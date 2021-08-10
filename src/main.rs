#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::{fs, io};

use clap::{AppSettings, Clap};
use indicatif::{MultiProgress, ParallelProgressIterator, ProgressBar};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rayon::prelude::*;

mod logger;
mod xml_parser;

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Discreater")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, about = "dataset directory", parse(from_os_str))]
    source_path: PathBuf,
    #[clap(short, long, about = "target directory", parse(from_os_str))]
    target_path: PathBuf,
    #[clap(short, long, about = "class number: full, medium or single", default_value = "full", parse(from_str = Cls::from_str))]
    cls: Cls,
}

#[derive(Debug, Copy, Clone)]
pub enum Cls {
    Full,
    Medium,
    Single,
}

impl Default for Cls {
    fn default() -> Self {
        Self::Full
    }
}

impl Cls {
    pub fn from_str(s: &str) -> Self {
        match s {
            "medium" => Self::Medium,
            "single" => Self::Single,
            _ => Self::Full,
        }
    }

    pub fn get_name<'a>(&self, s: &'a str) -> &'a str {
        match self {
            Cls::Full => s,
            // remove end digits
            Cls::Medium => &s[..s.chars().position(|c| c.is_ascii_digit()).unwrap()],
            Cls::Single => "dac_object",
        }
    }
}

fn main() -> io::Result<()> {
    let opts: Opts = Opts::parse();
    logger::setup_logger();
    assert!(opts.source_path.is_dir());
    if !opts.target_path.exists() {
        fs::create_dir_all(&opts.target_path)?;
    } else {
        assert!(opts.target_path.is_dir());
    }
    log::info!("{:?}", opts);
    convert(&opts.source_path, &opts.target_path, opts.cls)?;
    Ok(())
}

fn convert(source: &PathBuf, target: &PathBuf, cls: Cls) -> io::Result<()> {
    log::info!("converting...");
    let src: Vec<_> = fs::read_dir(source)?.collect();
    let mut files = vec![];
    // store category with it's id
    let mut category_map = BTreeMap::new();
    let mut category_num: usize = 0;
    let pb = logger::get_pb(src.len(), "reading dir");
    for obj in src {
        let obj = obj?.path();
        if obj.is_dir() {
            let obj_name = cls
                .get_name(obj.file_name().unwrap().to_str().unwrap())
                .to_owned();
            category_map.entry(obj_name).or_insert_with(|| {
                category_num += 1;
                category_num
            });
            let items = fs::read_dir(obj)?;
            for item in items {
                let item = item?.path();
                if item.is_file() && item.extension().unwrap() == "xml" {
                    files.push(item);
                }
            }
        }
        pb.inc(1);
    }
    let item_num = files.len();

    log::info!("shuffling {} items", item_num);
    let mut rng = rand::rngs::StdRng::seed_from_u64(233);
    files.shuffle(&mut rng);
    log::info!("shuffle finish");

    let m = Arc::new(MultiProgress::new());

    let train_num = (item_num as f64 * 0.8) as usize;

    let files = Arc::new(files);

    let target_anno = target.clone();
    let files_anno = Arc::clone(&files);
    let m_anno = Arc::clone(&m);
    let anno_gen: JoinHandle<io::Result<()>> = thread::spawn(move || {
        let (train_files, val_files) = files_anno.split_at(train_num);
        let anno_dir = target_anno.join("annotations");
        if !anno_dir.exists() {
            fs::create_dir(&anno_dir)?;
        }
        let pb = m_anno.add(logger::get_pb(
            train_files.len(),
            "parsing train annotations",
        ));
        generate_anno(
            train_files,
            cls,
            &category_map,
            &anno_dir.join("instances_train2017.json"),
            pb,
        )?;
        let pb = m_anno.add(logger::get_pb(val_files.len(), "parsing val annotations"));
        generate_anno(
            val_files,
            cls,
            &category_map,
            &anno_dir.join("instances_val2017.json"),
            pb,
        )?;
        Ok(())
    });

    let target_copy = target.clone();
    let files_copy = Arc::clone(&files);
    let m_copy = Arc::clone(&m);
    let copy_img: JoinHandle<io::Result<()>> = thread::spawn(move || {
        let (train_files, val_files) = files_copy.split_at(train_num);
        let train_dir = target_copy.join("train2017");
        if !train_dir.exists() {
            fs::create_dir(&train_dir)?;
        }
        let pb = m_copy.add(logger::get_pb(train_files.len(), "copying train"));
        copy_files(train_files, &train_dir, &pb)?;
        pb.finish();
        let val_dir = target_copy.join("val2017");
        if !val_dir.exists() {
            fs::create_dir(&val_dir)?;
        }
        let pb = m_copy.add(logger::get_pb(val_files.len(), "copying val"));
        copy_files(val_files, &val_dir, &pb)?;
        pb.finish();
        Ok(())
    });

    anno_gen.join().unwrap()?;
    copy_img.join().unwrap()?;

    log::info!("finished");

    Ok(())
}

fn generate_anno(
    files: &[PathBuf],
    cls: Cls,
    category_id: &BTreeMap<String, usize>,
    target_json_path: &PathBuf,
    pb: ProgressBar,
) -> io::Result<()> {
    let (images, annotations): (Vec<_>, Vec<_>) = files
        .par_iter()
        .progress_with(pb)
        .map(|file| {
            let img_file = file.with_extension("jpg");
            let res = (
                xml_parser::parse(file, cls).unwrap(),
                get_flatted_file_name(&img_file),
            );
            res
        })
        .enumerate()
        .map(|(id, (anno, img_name))| anno.into_json(id + 1, &img_name, category_id))
        .unzip();
    let info = serde_json::json!({
        "year": 2021,
        "version": "1.0",
        "description": "For object detection",
        "date_created": "2021"
    });
    let licenses = serde_json::json!([{
        "id": 1,
        "name": "GNU General Public License v3.0",
        "url": "https://github.com/zhiqwang/yolov5-rt-stack/blob/master/LICENSE",
    }]);
    let categories: Vec<_> = category_id
        .into_iter()
        .map(|(name, id)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "supercategory": name,
            })
        })
        .collect();
    let json = serde_json::json!({
        "info": info,
        "images": images,
        "licenses": licenses,
        "type": "instances",
        "annotations": annotations,
        "categories": categories
    });
    fs::write(target_json_path, json.to_string())?;
    Ok(())
}

fn get_flatted_file_name(file: &PathBuf) -> String {
    let img_name = file.file_name().unwrap().to_str().unwrap();
    let obj_name = file
        .parent()
        .unwrap()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    format!("{}_{}", obj_name, img_name)
}

fn copy_files(files: &[PathBuf], target_dir: &PathBuf, pb: &ProgressBar) -> io::Result<()> {
    for xml_file in files.iter() {
        let img_file = xml_file.with_extension("jpg");
        let new_file_name = get_flatted_file_name(&img_file);
        let new_file_path = target_dir.join(&new_file_name);
        fs::copy(img_file, new_file_path)?;
        pb.inc(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path;

    #[test]
    fn file_stem() {
        let f1 = path::Path::new("/a/b/c");
        assert_eq!(f1.file_stem(), Some(OsStr::new("c")));
    }
}
