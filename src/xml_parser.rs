use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use xml::reader::XmlEvent;
use xml::EventReader;

#[derive(Debug)]
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

#[derive(Debug)]
struct Size {
    width: usize,
    height: usize,
}

#[derive(Debug)]
struct Bndbox {
    xmin: usize,
    ymin: usize,
    xmax: usize,
    ymax: usize,
}

pub fn parse(file: &PathBuf, cls: super::Cls) -> std::io::Result<Annotataion> {
    let file = fs::File::open(file)?;
    let parser = EventReader::new(file);
    let content: Vec<_> = parser
        .into_iter()
        .filter_map(|p| {
            if let Ok(XmlEvent::Characters(s)) = p {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    let mut iter = content.into_iter();

    let annotation = Annotataion {
        filename: iter.next().unwrap(),
        size: Size {
            height: iter.next().unwrap().parse().unwrap(),
            width: iter.next().unwrap().parse().unwrap(),
        },
        object_name: cls.get_name(&iter.next().unwrap()).to_owned(),
        bndbox: Bndbox {
            xmin: iter.next().unwrap().parse().unwrap(),
            ymin: iter.next().unwrap().parse().unwrap(),
            xmax: iter.next().unwrap().parse().unwrap(),
            ymax: iter.next().unwrap().parse().unwrap(),
        },
    };

    Ok(annotation)
}

#[cfg(test)]
mod tests {
    use xml::reader::{Error, EventReader, XmlEvent};

    #[test]
    fn serde_xml() {
        let xml = r##"
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
        let parser = EventReader::new(xml.as_bytes());
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
