#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::f64;
use std::f64::consts::PI;
use std::cmp::Ordering;
use std::rc::Rc;
use std::cell::RefCell;

use geojson::GeoJson;
use std::collections::HashMap;
use serde_json::Value;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! println {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

fn body() -> web_sys::HtmlElement {
    document().body().expect("document should have a body")
}

fn canvas() -> web_sys::HtmlCanvasElement {
    let canvas = document().get_element_by_id("canvas").expect("document should have a canvas with id 'canvas'");
    canvas.dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap()
}

/// 用canvas绘制一个曲线动画——深入理解贝塞尔曲线
/// https://github.com/hujiulong/blog/issues/1
#[wasm_bindgen]
#[derive(Debug)]
pub struct Position {
    x: f64,
    y: f64
}

#[wasm_bindgen]
impl Position {
    pub fn new (x: f64, y: f64) -> Self {
        Position {
            x,
            y
        }
    }
}

/// Hue(色调)。0(或360)表示红色，120表示绿色，240表示蓝色，也可取其他数值来指定颜色。取值为：0 - 360
/// Saturation(饱和度)。取值为：0.0% - 100.0%
/// Lightness(亮度)。取值为：0.0% - 100.0%
/// Alpha透明度。取值0~1之间。
#[wasm_bindgen]
#[derive(Debug)]
pub struct HSL (u8, f32, f32);

#[wasm_bindgen]
impl HSL {
    pub fn new (hue: u8, saturation: f32, lightness: f32) -> Self {
        HSL (hue, saturation, lightness)
    }
}

impl HSL {
    fn as_str(&self) -> String {
        format!("hsl({}, {}%, {}%)", self.0, self.1 * 100.0, self.2 * 100.0)
    }
}

static EARTH_RAD:f64 = 6378137.0;

/// 经纬度转墨卡托
fn to_mercator(lng: f64, lat: f64) -> Position {
    let a = (lat * PI / 180.0).sin();
    Position {
        x: lng * PI / 180.0 * EARTH_RAD,
        y: EARTH_RAD / 2.0 * ((1.0 + a) / (1.0 - a)).ln()
    }
}

fn offset(pos: &Position) -> Position {
    Position {
        x: (pos.x - ORIGIN.x) / 7240.27140303,
        y: (ORIGIN.y - pos.y) / 7200.14089938,
    }
}

lazy_static! {
    static ref ORIGIN: Position = {
        to_mercator(73.50235, 53.56362)
    };

    /// 城市坐标映射表
    static ref CITIES: HashMap<String, Position> = {
        let mut m = HashMap::new();
        if let Ok(GeoJson::FeatureCollection(feature_collection)) = include_str!("datav.json").parse::<GeoJson>() {
            feature_collection.features.into_iter()
                .for_each(|feature| {
                    let properties = feature.properties.unwrap();
                    if let (Some(Value::String(name)), Some(Value::Array(center))) = (properties.get("name"), properties.get("center")) {
                        if let (Some(Value::Number(lng)), Some(Value::Number(lat))) = (center.first(), center.last()) {
                            if let (Some(lng), Some(lat)) = (lng.as_f64(), lat.as_f64()) {
                                m.insert(name.clone(), offset(&to_mercator(lng, lat)));
                            }
                        }
                    }
                });
        }
        m
    };
}

/// 通过起点/终点和曲率计算控制点
fn get_control_position (from: &Position, to: &Position, curveness: f64)  -> Position{
    return Position {
        x: (from.x + to.x) / 2.0 - (from.y - to.y) * curveness,
        y: (from.y + to.y) / 2.0 - (from.x - to.x) * curveness
    };
}

fn get_curveness (from: &Position, to: &Position) -> f64 {
    if from.y.partial_cmp(&to.y) == Some(Ordering::Less) {
        return 0.4
    }
    return -0.4
}

/// 绘制曲线路径的头部
fn draw_head_of_curve_path (ctx: &web_sys::CanvasRenderingContext2d, from: &Position, color: &HSL, radius: f64) {
    ctx.begin_path();
    ctx.set_fill_style(&JsValue::from(&color.as_str()));
    ctx.arc(from.x, from.y, radius, 0.0, 2.0 * PI);
    ctx.close_path();
    ctx.fill();
}

/// 绘制一条曲线路径的一部分
fn draw_part_of_curve_path (ctx: &web_sys::CanvasRenderingContext2d, from: &Position, to: &Position, curveness: f64, percent: f32) -> (Position, Position) {
    let cp = get_control_position(from, to, curveness);

    // 进度 100% 时，不进行计算，直接返回结果
    if 1.0.partial_cmp(&percent) == Some(Ordering::Less) {
        return (cp, Position{
            x: to.x, y: to.y
        })
    }

    let t = percent as f64;

    let p0 = from;
    let p1 = &cp;
    let p2 = to;
    let v01 = [ p1.x - p0.x, p1.y - p0.y ];     // 向量<p0, p1>
    let v12 = [ p2.x - p1.x, p2.y - p1.y ];     // 向量<p1, p2>
    let q0 = Position {
        x: p0.x + v01[0] * t,
        y: p0.y + v01[1] * t
    };
    let q1 = Position {
        x: p1.x + v12[0] * t,
        y: p1.y + v12[1] * t
    };

    let v = [ q1.x - q0.x, q1.y - q0.y ];       // 向量<q0, q1>

    let b = Position {
        x: q0.x + v[0] * t,
        y: q0.y + v[1] * t
    };

    (q0, b)
}

/// 绘制光晕 - 径向渐变
fn draw_hola (ctx: &web_sys::CanvasRenderingContext2d, pos: &Position, color: &HSL, radius: f64, percent: f32)  -> Result<(), JsValue> {
    let radius = radius * (percent as f64);
    let gradient = ctx.create_radial_gradient(pos.x, pos.y, 0.0, pos.x, pos.y, radius)?;

    gradient.add_color_stop(0.0, "transparent");
    gradient.add_color_stop(0.95, &color.as_str());
    gradient.add_color_stop(1.0, "transparent");

    ctx.set_fill_style(&gradient);
    ctx.fill_rect(pos.x - radius, pos.y - radius, pos.x + radius, pos.y + radius);

    Ok(())
}

/// 处理进度，使其合法化，0.0 ~ 1.0 之间。
pub fn normalize_process(number: f32) -> f32 {
    if 0.0.partial_cmp(&number) == Some(Ordering::Less) {
        if 1.0.partial_cmp(&number) == Some(Ordering::Greater) {
            return number;
        }
        return 1.0;
    }
    return 0.0;
}

static RADIUS: f64 = 20.0;

/// 绘制航线
#[wasm_bindgen]
pub fn draw_air_line (ctx: &web_sys::CanvasRenderingContext2d, from: &Position, to: &Position, color: &HSL, curveness: f64, percent: f32)  -> Result<(), JsValue>  {
    println!("{:?}", (from, to));
    let move_percent = 0.5;     // 移动和累积的占比，50% 为移动，其余为累积光晕
    let length_of_curve = 0.3;  // 尾巴长度占比

    let percent_head = normalize_process(percent / move_percent);   // 当前进度 / 移动的额度，用于转化为对应范围的百分比
    let percent_curve = normalize_process(percent - length_of_curve) / (move_percent + length_of_curve);   // 尾巴占 0% - 80%，长度为 30%
    let percent_hola = normalize_process(percent - move_percent) / (1.0 - move_percent); // 当前进度 - 移动的额度，之后转化成对应范围的百分比

    let (q0,b) = draw_part_of_curve_path(ctx, from, to,  curveness, percent_head);

    // 绘制光晕
    draw_hola(ctx, &b, color, RADIUS, percent_hola)?;

    // 绘制头部
    draw_head_of_curve_path(ctx, &b, color, 2.0);

    // 绘制尾巴
    if 0.0.partial_cmp(&percent_head) == Some(Ordering::Less) {
        ctx.move_to(from.x, from.y);
        ctx.quadratic_curve_to(q0.x,  q0.y,  b.x,  b.y);

        // 渐变颜色
        let gradient = ctx.create_linear_gradient(from.x,  from.y,  b.x,  b.y);
        gradient.add_color_stop(percent_curve, "transparent"); // transparent
        gradient.add_color_stop(1.0, &color.as_str());

        ctx.set_line_width(3.0);
        ctx.set_stroke_style(&gradient);
        ctx.stroke();
    }

    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let canvas = canvas();
    canvas.set_width(947);
    canvas.set_height(925);

    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    let color = vec!(
        HSL(255,1.0,1.0),
        HSL(128,1.0,0.5),
        HSL(255,1.0,0.5),
        HSL(50,1.0,0.5),
        HSL(170,1.0,0.5),
        HSL(180,1.0,0.5)
    );

    let from = vec!(
        CITIES.get("北京市").unwrap(),
        CITIES.get("北京市").unwrap(),
        CITIES.get("北京市").unwrap(),
        CITIES.get("广东省").unwrap(),
        CITIES.get("北京市").unwrap()
    );

    let to = vec!(
        CITIES.get("广西壮族自治区").unwrap(),
        CITIES.get("广东省").unwrap(),
        CITIES.get("吉林省").unwrap(),
        CITIES.get("新疆维吾尔自治区").unwrap(),
        CITIES.get("新疆维吾尔自治区").unwrap()
    );


    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    let mut i = 0;

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
//        if i > 50 {
//            let _ = f.borrow_mut().take();
//            return;
//        }

        context.clear_rect(0.0, 0.0, 1000.0, 1000.0);
        i += 1;
        for j in 0..5 {
            draw_air_line(&context, from[j], to[j], &color[j], get_curveness(from[j], to[j]), ((i + j * 20) % 100) as f32 / 100.0);
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}