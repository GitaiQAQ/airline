#[macro_use]
extern crate lazy_static;

use geojson::GeoJson;
use std::iter::Map;
use std::collections::HashMap;
use serde_json::Value;
use std::f64::consts::PI;

#[derive(Debug)]
pub struct Position {
    x: f64,
    y: f64
}

lazy_static! {
    static ref DATAV: HashMap<String, Position> = {
        let mut m = HashMap::new();
        if let Ok(GeoJson::FeatureCollection(feature_collection)) = include_str!("datav.json").parse::<GeoJson>() {
            feature_collection.features.into_iter()
                .for_each(|feature| {
                    let properties = feature.properties.unwrap();
                    if let (Some(Value::String(name)), Some(Value::Array(center))) = (properties.get("name"), properties.get("center")) {

                        if let (Some(Value::Number(x)), Some(Value::Number(y))) = (center.first(), center.last()) {
                            if let (Some(x), Some(y)) = (x.as_f64(), y.as_f64()) {
                                unsafe {
                                    m.insert(name.clone(), Position {
                                        x: x.clone(),
                                        y: y.clone()
                                    });
                                }
                            }
                        }
                    }
                });
        }
        m
    };
}

static EARTH_RAD:f64 = 6378137.0;

/// 经纬度转墨卡托
/// 原点： [8182244.174108973, 7087940.533366477]
/// 新疆维吾尔自治区，克孜勒苏柯尔克孜自治州，阿克陶县 [73.50235, 39.38378], [8182244.174108973, 4776794.810287128]
/// 黑龙江省，漠河县，大兴安岭地区 [123.28041, 53.56362], [13723512.465985991, 7087940.533366477]
/// 黑龙江省，佳木斯市，抚远市 [135.09567, 48.437518], [15038781.192776127, 6179953.3588861255]
/// 海南省，三亚市，[112.055295, 3.840206]，[12473938.380090054, 427810.20143990475 ]
/// 宽度：135.09567 - 73.50235   15038781.192776127-8182244.174108973=6856537.01867
/// 高度：53.56362-3.840206      7087940.533366477-427810.20143990475=6660130.33193
/// 7240.27140303  7200.14089938
fn to_mercator(lng: f64, lat: f64) -> Position {
    let a = (lat * PI / 180.0).sin();
    Position {
        x: lng * PI / 180.0 * EARTH_RAD,
        y: EARTH_RAD / 2.0 * ((1.0 + a) / (1.0 - a)).ln()
    }
}

pub fn main () {
    println!("海南省，三亚市 {:?}", to_mercator(112.055295, 3.840206));
    println!("原点 {:?}", to_mercator(61.59332, 49.723414));
}