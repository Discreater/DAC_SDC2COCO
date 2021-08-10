use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{fs, io};

use xml::reader::{Events, XmlEvent};
use xml::EventReader;

#[derive(Debug, Default)]
pub struct Annotataion {
    filename: String,
    size: Size,
    object_name: String,
    bndbox: Bndbox,
}

impl Annotataion {
    /// one image one object, so id is same.
    pub fn into_json(
        self,
        id: usize,
        img_name: &str,
        category_id: &BTreeMap<String, usize>,
    ) -> (serde_json::Value, serde_json::Value) {
        let Bndbox {
            xmin,
            ymin,
            xmax,
            ymax,
        } = self.bndbox;
        let Size { height, width } = self.size;
        let box_w = xmax - xmin;
        let box_h = ymax - ymin;

        let image = serde_json::json!({
            "date_captured": "2021",
            "file_name": img_name,
            "id": id,
            "height": height,
            "width": width,
        });

        let anno = serde_json::json!({
            "segmentation": [[xmin, ymin, xmax, ymin, xmax, ymax, xmin, ymax]],
            "area": box_w * box_h,
            "iscrowd": 0,
            "image_id": id,
            "bbox": [
                xmin, ymin, box_w, box_h,
            ],
            "category_id": category_id[&self.object_name],
            "id": id
        });

        (image, anno)
    }
}

#[derive(Debug, Default)]
struct Size {
    width: usize,
    height: usize,
}

#[derive(Debug, Default)]
struct Bndbox {
    xmin: usize,
    ymin: usize,
    xmax: usize,
    ymax: usize,
}

pub fn parse(file_path: &PathBuf, cls: super::Cls) -> std::io::Result<Annotataion> {
    let file = fs::File::open(file_path)?;
    let parser = EventReader::new(file);

    let mut iter = parser.into_iter();

    let mut anno = Annotataion::default();
    while let Some(next) = iter.next() {
        let next = next.unwrap();
        if let XmlEvent::StartElement { name, .. } = next {
            match &name.local_name[..] {
                "filename" => {
                    anno.filename = get_next_characters(&mut iter, file_path);
                }
                "width" => {
                    anno.size.width = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                "height" => {
                    anno.size.height = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                "name" => {
                    anno.object_name = cls
                        .get_name(&get_next_characters(&mut iter, file_path))
                        .to_owned();
                }
                "xmin" => {
                    anno.bndbox.xmin = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                "xmax" => {
                    anno.bndbox.xmax = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                "ymin" => {
                    anno.bndbox.ymin = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                "ymax" => {
                    anno.bndbox.ymax = get_next_characters(&mut iter, file_path).parse().unwrap();
                }
                _ => {}
            }
        }
    }
    Ok(anno)
}

fn get_next_characters<T: io::Read>(iter: &mut Events<T>, file_path: &PathBuf) -> String {
    loop {
        let next = iter.next();
        if let Some(Ok(XmlEvent::Characters(s))) = next {
            break s;
        } else if next == None {
            panic!("paring error, file: {:?}", file_path);
        }
    }
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use xml::reader::{EventReader, XmlEvent};

    use crate::Cls;

    use super::{get_next_characters, Annotataion};
    const XML: &'static str = r##"
            <annotation>
                <filename>720 (13)_0001</filename>
                <size>
                    <width>640</width>
                    <height>360</height>
                </size>
                <object>
                    <name>whale1</name>
                    <bndbox>
                        <xmin>276</xmin>
                        <ymin>128</ymin>
                        <xmax>311</xmax>
                        <ymax>194</ymax>
                    </bndbox>
                </object>
            </annotation>
        "##;

    #[test]
    fn anno_parse() {
        let file_path = &PathBuf::new().join("a/b");
        let cls = Cls::Single;
        let parser = EventReader::new(XML.as_bytes());

        let mut iter = parser.into_iter();

        let mut anno = Annotataion::default();
        while let Some(next) = iter.next() {
            if let Ok(XmlEvent::StartElement { name, .. }) = next {
                println!("{:?}", name);
                match &name.local_name[..] {
                    "filename" => {
                        anno.filename = get_next_characters(&mut iter, file_path);
                    }
                    "width" => {
                        anno.size.width =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    "height" => {
                        anno.size.height =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    "name" => {
                        anno.object_name = cls
                            .get_name(&get_next_characters(&mut iter, file_path))
                            .to_owned();
                    }
                    "xmin" => {
                        anno.bndbox.xmin =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    "xmax" => {
                        anno.bndbox.xmax =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    "ymin" => {
                        anno.bndbox.ymin =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    "ymax" => {
                        anno.bndbox.ymax =
                            get_next_characters(&mut iter, file_path).parse().unwrap();
                    }
                    _ => {}
                }
            }
        }

        dbg!(anno);
    }

    #[test]
    fn serde_xml() {
        let parser = EventReader::new(XML.as_bytes());
        let mut depth = 0;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, .. }) => {
                    println!("{}+{}", indent(depth), name);
                    depth += 1;
                }
                Ok(XmlEvent::Characters(s)) => {
                    println!("{}@{}", indent(depth), s);
                }
                Ok(XmlEvent::EndElement { name }) => {
                    depth -= 1;
                    println!("{}-{}", indent(depth), name);
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        fn indent(size: usize) -> String {
            const INDENT: &'static str = "    ";
            (0..size)
                .map(|_| INDENT)
                .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
        }
    }
}
