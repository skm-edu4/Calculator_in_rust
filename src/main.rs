use eframe::egui::{self, Color32};

const GAP: f32 = 8.0;
const ROW_H: f32 = 68.0;
const HIST_W: f32 = 250.0;

const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
const DIGIT_BG: Color32 = Color32::from_rgb(51, 51, 51);
const TOP_BG: Color32 = Color32::from_rgb(165, 165, 165);
const ORANGE: Color32 = Color32::from_rgb(255, 159, 10);
const SEG_BG: Color32 = Color32::from_rgb(44, 44, 48);
const DIM: Color32 = Color32::from_rgb(140, 140, 146);
const DARK_TEXT: Color32 = Color32::from_rgb(20, 20, 20);
const DISABLED_BG: Color32 = Color32::from_rgb(30, 30, 34);
const DISABLED_FG: Color32 = Color32::from_rgb(80, 80, 86);

const BASIC_SIZE: (f32, f32) = (300.0, 540.0);
const SCI_SIZE: (f32, f32) = (660.0, 540.0);
const PROG_SIZE: (f32, f32) = (660.0, 630.0);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([BASIC_SIZE.0, BASIC_SIZE.1])
            .with_resizable(false)
            .with_title("Calculator"),
        ..Default::default()
    };
    eframe::run_native(
        "Calculator",
        options,
        Box::new(|cc| Ok(Box::new(CalcApp::new(cc)))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Basic,
    Scientific,
    Programmer,
}

impl Mode {
    fn size(self) -> (f32, f32) {
        match self {
            Mode::Basic => BASIC_SIZE,
            Mode::Scientific => SCI_SIZE,
            Mode::Programmer => PROG_SIZE,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Digit,
    Top,
    Op,
    Eq,
    Fn,
    FnHot,
    Disabled,
    Seg,
    SegOn,
}

fn kind_style(k: Kind) -> (Color32, Color32) {
    match k {
        Kind::Digit | Kind::Fn => (DIGIT_BG, Color32::WHITE),
        Kind::Top => (TOP_BG, DARK_TEXT),
        Kind::Op | Kind::Eq | Kind::FnHot | Kind::SegOn => (ORANGE, Color32::WHITE),
        Kind::Disabled => (DISABLED_BG, DISABLED_FG),
        Kind::Seg => (SEG_BG, DIM),
    }
}

fn shade(col: Color32, f: f32) -> Color32 {
    let m = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(col.r()), m(col.g()), m(col.b()))
}

fn draw_button(ui: &mut egui::Ui, label: &str, kind: Kind, w: f32, h: f32, font: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let (bg, fg) = kind_style(kind);
    let fill = if resp.is_pointer_button_down_on() {
        shade(bg, 0.7)
    } else if resp.hovered() && kind != Kind::Disabled {
        shade(bg, 1.15)
    } else {
        bg
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same((h * 0.26) as u8), fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font),
        fg,
    );
    resp.clicked() && kind != Kind::Disabled
}

type Cell = (f32, &'static str, Kind, f32, Act);

fn draw_row(ui: &mut egui::Ui, w: f32, cells: &[Cell], h: f32) -> Option<Act> {
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        let n = cells.len() as f32;
        let total_weight: f32 = cells.iter().map(|c| c.0).sum();
        let usable = (w - GAP * (n - 1.0)).max(10.0) - 1.0;
        for (weight, label, kind, font, act) in cells {
            let cw = usable * weight / total_weight;
            if draw_button(ui, label, *kind, cw, h, *font) {
                clicked = Some(*act);
            }
        }
    });
    clicked
}

fn draw_column(ui: &mut egui::Ui, w: f32, rows: &[Vec<Cell>], h: f32) -> Option<Act> {
    let mut clicked = None;
    for row in rows {
        if let Some(act) = draw_row(ui, w, row, h) {
            clicked = Some(act);
        }
    }
    clicked
}

fn group(s: &str) -> String {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s),
    };
    let (int, frac) = match rest.find('.') {
        Some(i) => (&rest[..i], Some(&rest[i..])),
        None => (rest, None),
    };
    let mut out = String::new();
    let chars: Vec<char> = int.chars().collect();
    for (idx, ch) in chars.iter().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    let out: String = out.chars().rev().collect();
    format!("{}{}{}", sign, out, frac.unwrap_or(""))
}

fn format_f64(v: f64) -> String {
    if !v.is_finite() {
        return "Error".to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    if v.abs() >= 1e16 || v.abs() < 1e-9 {
        return format!("{:e}", v);
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return group(&format!("{}", v as i64));
    }
    let mut s = format!("{:.*}", 14, v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    group(&s)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
}

impl Op {
    fn prec(self) -> u8 {
        match self {
            Op::Add | Op::Sub => 1,
            Op::Mul | Op::Div | Op::Mod => 2,
            Op::Pow => 3,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "−",
            Op::Mul => "×",
            Op::Div => "÷",
            Op::Pow => "^",
            Op::Mod => "mod",
        }
    }
}

fn expr_from(terms: &[f64], ops: &[Op]) -> String {
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format_f64(*t));
        if let Some(op) = ops.get(i) {
            out.push(' ');
            out.push_str(op.symbol());
        }
    }
    out
}

fn apply_op(op: Op, a: f64, b: f64) -> Result<f64, String> {
    match op {
        Op::Add => Ok(a + b),
        Op::Sub => Ok(a - b),
        Op::Mul => Ok(a * b),
        Op::Div => {
            if b == 0.0 {
                Err("div0".to_string())
            } else {
                Ok(a / b)
            }
        }
        Op::Pow => Ok(a.powf(b)),
        Op::Mod => {
            if b == 0.0 {
                Err("mod0".to_string())
            } else {
                Ok(a % b)
            }
        }
    }
}

fn eval_expr(terms: &[f64], ops: &[Op]) -> Result<f64, String> {
    if terms.is_empty() {
        return Ok(0.0);
    }
    if terms.len() != ops.len() + 1 {
        return Err("shape".to_string());
    }
    let mut vals = terms.to_vec();
    let mut ops = ops.to_vec();
    for prec in [3u8, 2u8, 1u8] {
        let mut i = 0;
        while i < ops.len() {
            if ops[i].prec() == prec {
                let r = apply_op(ops[i], vals[i], vals[i + 1])?;
                vals[i] = r;
                vals.remove(i + 1);
                ops.remove(i);
            } else {
                i += 1;
            }
        }
    }
    Ok(vals[0])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FnKind {
    Sin,
    Cos,
    Tan,
    Ln,
    Log,
    Sqrt,
    Inv,
    Square,
    Exp,
    Exp10,
    Fact,
}

struct Frame {
    terms: Vec<f64>,
    ops: Vec<Op>,
}

struct Engine {
    terms: Vec<f64>,
    ops: Vec<Op>,
    entry: String,
    typing: bool,
    fresh: bool,
    error: bool,
    last: Option<(Vec<f64>, Vec<Op>)>,
    last_expr: String,
    frames: Vec<Frame>,
    angle_deg: bool,
    second: bool,
}

impl Engine {
    fn new() -> Self {
        Engine {
            terms: Vec::new(),
            ops: Vec::new(),
            entry: String::new(),
            typing: false,
            fresh: true,
            error: false,
            last: None,
            last_expr: String::new(),
            frames: Vec::new(),
            angle_deg: true,
            second: false,
        }
    }

    fn awaiting(&self) -> bool {
        self.terms.len() == self.ops.len()
    }

    fn parse_entry(&self) -> f64 {
        if self.entry.is_empty() {
            return 0.0;
        }
        if self.entry == "-" {
            return -0.0;
        }
        self.entry.parse::<f64>().unwrap_or(0.0)
    }

    fn current(&self) -> f64 {
        if self.typing {
            self.parse_entry()
        } else {
            self.terms.last().copied().unwrap_or(0.0)
        }
    }

    fn reset_soft(&mut self) {
        self.terms.clear();
        self.ops.clear();
        self.entry.clear();
        self.typing = false;
    }

    fn begin_number(&mut self) {
        self.last_expr.clear();
        if self.fresh {
            self.reset_soft();
        } else if !self.awaiting() {
            self.terms.pop();
        }
        self.fresh = false;
    }

    fn digit(&mut self, c: char) {
        if self.error {
            self.full_clear();
        }
        if !self.typing {
            self.begin_number();
        }
        let digits = self.entry.chars().filter(|c| c.is_ascii_digit()).count();
        if digits >= 16 {
            return;
        }
        let bare = self.entry.trim_start_matches('-');
        if bare == "0" && c == '0' {
            if self.entry.is_empty() {
                self.entry.push('0');
            }
            return;
        }
        if bare == "0" {
            let sign = if self.entry.starts_with('-') { "-" } else { "" };
            self.entry = format!("{}{}", sign, c);
        } else {
            self.entry.push(c);
        }
        self.typing = true;
    }

    fn dot(&mut self) {
        if self.error {
            self.full_clear();
        }
        if !self.typing {
            self.begin_number();
            self.entry.push_str("0.");
            self.typing = true;
            return;
        }
        if self.entry.is_empty() || self.entry == "-" {
            self.entry.push_str("0.");
        } else if !self.entry.contains('.') {
            self.entry.push('.');
        }
    }

    fn backspace(&mut self) {
        if self.error {
            return;
        }
        if self.typing {
            self.entry.pop();
        }
    }

    fn commit_entry(&mut self) {
        if self.typing {
            let v = self.parse_entry();
            self.terms.push(v);
            self.entry.clear();
            self.typing = false;
        }
    }

    fn press_op(&mut self, op: Op) {
        if self.error {
            return;
        }
        self.commit_entry();
        self.fresh = false;
        if self.terms.is_empty() {
            return;
        }
        if self.awaiting()
            && let Some(last) = self.ops.last_mut()
        {
            *last = op;
            return;
        }
        if let Some(&last) = self.ops.last()
            && op.prec() <= last.prec()
        {
            match eval_expr(&self.terms, &self.ops) {
                Ok(v) => {
                    self.terms = vec![v];
                    self.ops.clear();
                }
                Err(_) => {
                    self.error = true;
                    return;
                }
            }
        }
        self.ops.push(op);
    }

    fn equals(&mut self) -> Option<f64> {
        if self.error {
            return None;
        }
        if !self.typing && self.fresh {
            if let Some((ref lt, ref lo)) = self.last
                && !lt.is_empty()
            {
                let base = self.current();
                let mut nt = vec![base];
                nt.extend_from_slice(&lt[1..]);
                match eval_expr(&nt, lo) {
                    Ok(v) => {
                        if !v.is_finite() {
                            self.error = true;
                            return None;
                        }
                        self.terms = vec![v];
                        return None;
                    }
                    Err(_) => {
                        self.error = true;
                        return None;
                    }
                }
            }
            return None;
        }
        self.commit_entry();
        if self.terms.is_empty() {
            return None;
        }
        if self.awaiting() {
            let last = self.terms.last().copied().unwrap_or(0.0);
            self.terms.push(last);
        }
        let expr = expr_from(&self.terms, &self.ops);
        let had_ops = !self.ops.is_empty();
        match eval_expr(&self.terms, &self.ops) {
            Ok(v) => {
                if !v.is_finite() {
                    self.error = true;
                    return None;
                }
                self.last = Some((self.terms.clone(), self.ops.clone()));
                self.terms = vec![v];
                self.ops.clear();
                self.entry.clear();
                self.typing = false;
                self.fresh = true;
                self.last_expr = format!("{} =", expr);
                had_ops.then_some(v)
            }
            Err(_) => {
                self.error = true;
                None
            }
        }
    }

    fn clear_label(&self) -> &'static str {
        let content = self.error || self.typing || !self.entry.is_empty() || self.current() != 0.0;
        if content { "C" } else { "AC" }
    }

    fn clear_press(&mut self) {
        if self.clear_label() == "AC" {
            self.full_clear();
            return;
        }
        self.error = false;
        self.second = false;
        self.last_expr.clear();
        if self.typing {
            self.terms.push(0.0);
            self.entry.clear();
            self.typing = false;
        } else if let Some(t) = self.terms.last_mut() {
            *t = 0.0;
        } else {
            self.terms.push(0.0);
        }
        self.fresh = false;
    }

    fn full_clear(&mut self) {
        self.terms.clear();
        self.ops.clear();
        self.entry.clear();
        self.typing = false;
        self.fresh = true;
        self.error = false;
        self.last = None;
        self.last_expr.clear();
        self.frames.clear();
        self.second = false;
    }

    fn negate(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        if self.typing {
            if let Some(stripped) = self.entry.strip_prefix('-') {
                self.entry = stripped.to_string();
            } else {
                if self.entry.is_empty() {
                    self.entry.push('0');
                }
                self.entry.insert(0, '-');
            }
        } else if let Some(t) = self.terms.last_mut() {
            *t = -*t;
        }
    }

    fn percent(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        let x = self.current();
        let left = if self.terms.len() >= 2 {
            self.terms[self.terms.len() - 2]
        } else {
            self.terms.last().copied().unwrap_or(0.0)
        };
        let pct = match self.ops.last() {
            Some(Op::Add) | Some(Op::Sub) => left * x / 100.0,
            _ => x / 100.0,
        };
        self.entry.clear();
        if self.typing {
            self.typing = false;
            self.terms.push(pct);
        } else if let Some(t) = self.terms.last_mut() {
            *t = pct;
        } else {
            self.terms.push(pct);
        }
        self.fresh = false;
    }

    fn constant(&mut self, v: f64) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        self.entry.clear();
        self.typing = false;
        if self.terms.is_empty() || self.awaiting() {
            self.terms.push(v);
        } else if let Some(t) = self.terms.last_mut() {
            *t = v;
        }
        self.fresh = false;
    }

    fn unary(&mut self, kind: FnKind) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        let x = self.current();
        let second = self.second;
        self.second = false;
        let r = match kind {
            FnKind::Sin | FnKind::Cos | FnKind::Tan => {
                let trig = match kind {
                    FnKind::Sin => f64::sin,
                    FnKind::Cos => f64::cos,
                    _ => f64::tan,
                };
                let inv = match kind {
                    FnKind::Sin => f64::asin,
                    FnKind::Cos => f64::acos,
                    _ => f64::atan,
                };
                if second {
                    let v = inv(x);
                    if self.angle_deg { v.to_degrees() } else { v }
                } else {
                    let a = if self.angle_deg { x.to_radians() } else { x };
                    trig(a)
                }
            }
            FnKind::Ln => x.ln(),
            FnKind::Log => x.log10(),
            FnKind::Sqrt => x.sqrt(),
            FnKind::Inv => 1.0 / x,
            FnKind::Square => x * x,
            FnKind::Exp => x.exp(),
            FnKind::Exp10 => 10f64.powf(x),
            FnKind::Fact => {
                if x < 0.0 || x.fract() != 0.0 || x > 170.0 {
                    self.error = true;
                    return;
                }
                let mut acc = 1.0;
                for i in 2..=(x as i64) {
                    acc *= i as f64;
                }
                acc
            }
        };
        if !r.is_finite() {
            self.error = true;
            return;
        }
        self.entry.clear();
        self.typing = false;
        if self.terms.is_empty() {
            self.terms.push(r);
        } else if let Some(t) = self.terms.last_mut() {
            *t = r;
        }
        self.fresh = false;
    }

    fn open_paren(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        self.commit_entry();
        self.frames.push(Frame {
            terms: std::mem::take(&mut self.terms),
            ops: std::mem::take(&mut self.ops),
        });
        self.fresh = true;
    }

    fn close_paren(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        let frame = match self.frames.pop() {
            Some(f) => f,
            None => return,
        };
        self.commit_entry();
        if self.terms.is_empty() {
            self.terms.push(0.0);
        }
        if self.awaiting() {
            let last = self.terms.last().copied().unwrap_or(0.0);
            self.terms.push(last);
        }
        let v = match eval_expr(&self.terms, &self.ops) {
            Ok(v) => v,
            Err(_) => {
                self.error = true;
                return;
            }
        };
        self.terms = frame.terms;
        self.ops = frame.ops;
        if self.terms.is_empty() || self.awaiting() {
            self.terms.push(v);
        } else if let Some(t) = self.terms.last_mut() {
            *t = v;
        }
        self.entry.clear();
        self.typing = false;
        self.fresh = false;
    }

    fn live_expression(&self) -> Option<String> {
        if self.ops.is_empty() && self.frames.is_empty() {
            return None;
        }
        if self.terms.is_empty() && self.ops.is_empty() && !self.typing {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();
        for _ in &self.frames {
            parts.push("(".to_string());
        }
        for (i, t) in self.terms.iter().enumerate() {
            parts.push(format_f64(*t));
            if let Some(op) = self.ops.get(i) {
                parts.push(op.symbol().to_string());
            }
        }
        if self.typing && !self.entry.is_empty() {
            parts.push(group(&self.entry));
        }
        Some(parts.join(" "))
    }

    fn expression_string(&self) -> String {
        if self.error {
            return String::new();
        }
        self.live_expression()
            .unwrap_or_else(|| self.last_expr.clone())
    }

    fn display_string(&self) -> String {
        if self.error {
            return "Error".to_string();
        }
        if self.typing {
            if self.entry.is_empty() {
                return "0".to_string();
            }
            if self.entry == "-" {
                return "-0".to_string();
            }
            return group(&self.entry);
        }
        match self.terms.last() {
            Some(v) => format_f64(*v),
            None => "0".to_string(),
        }
    }

    fn copy_text(&self) -> String {
        if self.error {
            return "Error".to_string();
        }
        if self.typing {
            self.entry.clone()
        } else {
            format_f64(self.current()).replace(',', "")
        }
    }

    fn paste(&mut self, text: &str) {
        let cleaned: String = text.trim().replace(',', "");
        if let Ok(v) = cleaned.parse::<f64>() {
            if !v.is_finite() {
                return;
            }
            self.full_clear();
            self.entry = format!("{}", v);
            self.typing = true;
            self.fresh = false;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Base {
    Bin,
    Oct,
    Dec,
    Hex,
}

impl Base {
    fn radix(self) -> u32 {
        match self {
            Base::Bin => 2,
            Base::Oct => 8,
            Base::Dec => 10,
            Base::Hex => 16,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Base::Bin => "BIN",
            Base::Oct => "OCT",
            Base::Dec => "DEC",
            Base::Hex => "HEX",
        }
    }
}

fn fmt_int_raw(v: i64, base: Base) -> String {
    if base == Base::Dec {
        return format!("{}", v);
    }
    let sign = if v < 0 { "-" } else { "" };
    let mag = v.unsigned_abs();
    let s = match base {
        Base::Hex => format!("{:X}", mag),
        Base::Oct => format!("{:o}", mag),
        Base::Bin => format!("{:b}", mag),
        Base::Dec => format!("{}", mag),
    };
    format!("{}{}", sign, s)
}

fn fmt_int(v: i64, base: Base) -> String {
    if base == Base::Dec {
        return group(&format!("{}", v));
    }
    fmt_int_raw(v, base)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum POp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

impl POp {
    fn symbol(self) -> &'static str {
        match self {
            POp::Add => "+",
            POp::Sub => "−",
            POp::Mul => "×",
            POp::Div => "÷",
            POp::And => "AND",
            POp::Or => "OR",
            POp::Xor => "XOR",
            POp::Shl => "Lsh",
            POp::Shr => "Rsh",
        }
    }

    fn apply(self, l: i64, r: i64) -> Result<i64, String> {
        match self {
            POp::Add => Ok(l.wrapping_add(r)),
            POp::Sub => Ok(l.wrapping_sub(r)),
            POp::Mul => Ok(l.wrapping_mul(r)),
            POp::Div => l.checked_div(r).ok_or_else(|| "div0".to_string()),
            POp::And => Ok(l & r),
            POp::Or => Ok(l | r),
            POp::Xor => Ok(l ^ r),
            POp::Shl => Ok(l.wrapping_shl(r as u32)),
            POp::Shr => Ok(l.wrapping_shr(r as u32)),
        }
    }
}

struct Programmer {
    base: Base,
    value: i64,
    entry: String,
    typing: bool,
    pending: Option<(POp, i64)>,
    last: Option<(POp, i64)>,
    last_expr: String,
    error: bool,
    fresh: bool,
    zeroed: bool,
}

impl Programmer {
    fn new() -> Self {
        Programmer {
            base: Base::Dec,
            value: 0,
            entry: String::new(),
            typing: false,
            pending: None,
            last: None,
            last_expr: String::new(),
            error: false,
            fresh: true,
            zeroed: false,
        }
    }

    fn current(&self) -> i64 {
        if self.typing {
            i64::from_str_radix(&self.entry, self.base.radix()).unwrap_or(0)
        } else {
            self.value
        }
    }

    fn commit(&mut self) {
        if self.typing {
            self.value = i64::from_str_radix(&self.entry, self.base.radix()).unwrap_or(0);
            self.entry.clear();
            self.typing = false;
        }
    }

    fn digit(&mut self, c: char) {
        if self.error {
            self.full_clear();
        }
        if c.to_digit(self.base.radix()).is_none() {
            return;
        }
        if self.entry.chars().count() >= 64 {
            return;
        }
        if !self.typing {
            self.last_expr.clear();
            self.entry.clear();
            self.typing = true;
            if self.fresh {
                self.value = 0;
                self.pending = None;
            }
            self.fresh = false;
        }
        self.zeroed = false;
        self.entry.push(c.to_ascii_uppercase());
    }

    fn backspace(&mut self) {
        if self.error {
            return;
        }
        if self.typing {
            self.entry.pop();
        }
    }

    fn press_op(&mut self, op: POp) {
        if self.error {
            return;
        }
        self.commit();
        self.zeroed = false;
        let left = match self.pending.take() {
            Some((p, l)) => match p.apply(l, self.value) {
                Ok(v) => v,
                Err(_) => {
                    self.error = true;
                    return;
                }
            },
            None => self.value,
        };
        self.pending = Some((op, left));
        self.fresh = false;
    }

    fn equals(&mut self) -> Option<i64> {
        if self.error {
            return None;
        }
        let had_entry = self.typing;
        self.commit();
        if let Some((op, left)) = self.pending.take() {
            let rhs = if had_entry {
                self.value
            } else if self.zeroed {
                0
            } else {
                self.last.map(|(_, r)| r).unwrap_or(left)
            };
            let expr = format!(
                "{} {} {}",
                fmt_int(left, self.base),
                op.symbol(),
                fmt_int(rhs, self.base)
            );
            match op.apply(left, rhs) {
                Ok(v) => {
                    self.value = v;
                    self.last = Some((op, rhs));
                    self.last_expr = format!("{} =", expr);
                    self.zeroed = false;
                    self.entry.clear();
                    self.typing = false;
                    self.fresh = true;
                    return Some(v);
                }
                Err(_) => {
                    self.error = true;
                    return None;
                }
            }
        }
        if self.fresh {
            if let Some((op, rhs)) = self.last {
                match op.apply(self.value, rhs) {
                    Ok(v) => self.value = v,
                    Err(_) => {
                        self.error = true;
                        return None;
                    }
                }
            }
        } else {
            self.value = self.current();
        }
        self.zeroed = false;
        self.entry.clear();
        self.typing = false;
        self.fresh = true;
        None
    }

    fn negate(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        self.zeroed = false;
        if self.typing {
            if let Some(stripped) = self.entry.strip_prefix('-') {
                self.entry = stripped.to_string();
            } else {
                self.entry.insert(0, '-');
            }
            if let Ok(v) = i64::from_str_radix(&self.entry, self.base.radix()) {
                self.entry = fmt_int_raw(v, self.base);
            }
        } else {
            self.value = self.value.wrapping_neg();
        }
    }

    fn percent(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        let cur = self.current();
        let pct = match self.pending {
            Some((POp::Add, l)) | Some((POp::Sub, l)) => l.wrapping_mul(cur) / 100,
            _ => cur / 100,
        };
        self.entry = fmt_int_raw(pct, self.base);
        self.typing = true;
        self.fresh = false;
        self.zeroed = false;
    }

    fn not(&mut self) {
        if self.error {
            return;
        }
        self.last_expr.clear();
        self.commit();
        self.value = !self.value;
        self.fresh = false;
    }

    fn set_base(&mut self, base: Base) {
        if base == self.base {
            return;
        }
        if self.pending.is_none() {
            self.last_expr.clear();
        }
        self.commit();
        self.base = base;
    }

    fn clear_label(&self) -> &'static str {
        let content = self.error
            || (!self.entry.is_empty() && self.entry != "0")
            || self.value != 0
            || self.pending.is_some();
        if content { "C" } else { "AC" }
    }

    fn clear_press(&mut self) {
        if self.clear_label() == "AC" {
            self.full_clear();
            return;
        }
        self.error = false;
        self.last_expr.clear();
        if self.typing {
            self.entry.clear();
            self.typing = false;
            self.value = 0;
        } else {
            self.value = 0;
        }
        self.zeroed = true;
        self.fresh = false;
    }

    fn full_clear(&mut self) {
        self.value = 0;
        self.entry.clear();
        self.typing = false;
        self.pending = None;
        self.last = None;
        self.last_expr.clear();
        self.error = false;
        self.fresh = true;
        self.zeroed = false;
    }

    fn live_expression(&self) -> Option<String> {
        let (op, left) = self.pending?;
        let mut s = format!("{} {}", fmt_int(left, self.base), op.symbol());
        if self.typing && !self.entry.is_empty() {
            s.push(' ');
            if self.base == Base::Dec {
                s.push_str(&group(&self.entry));
            } else {
                s.push_str(&self.entry);
            }
        }
        Some(s)
    }

    fn expression_string(&self) -> String {
        if self.error {
            return String::new();
        }
        self.live_expression()
            .unwrap_or_else(|| self.last_expr.clone())
    }

    fn display_string(&self) -> String {
        if self.error {
            return "Error".to_string();
        }
        if self.typing {
            if self.entry.is_empty() {
                return "0".to_string();
            }
            if self.base == Base::Dec {
                return group(&self.entry);
            }
            return self.entry.clone();
        }
        fmt_int(self.value, self.base)
    }

    fn secondary_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for b in [Base::Hex, Base::Oct, Base::Dec, Base::Bin] {
            if b != self.base {
                out.push(format!("{} {}", b.label(), fmt_int(self.value, b)));
            }
        }
        out
    }

    fn copy_text(&self) -> String {
        if self.error {
            return "Error".to_string();
        }
        if self.typing {
            self.entry.clone()
        } else {
            fmt_int_raw(self.value, self.base)
        }
    }

    fn paste(&mut self, text: &str) {
        let t = text.trim();
        let parsed = i64::from_str_radix(t, self.base.radix())
            .or_else(|_| t.replace(',', "").parse::<i64>());
        if let Ok(v) = parsed {
            self.full_clear();
            self.value = v;
            self.fresh = false;
        }
    }
}

#[derive(Clone, Copy)]
enum Act {
    Digit(char),
    Dot,
    Op(Op),
    Percent,
    Eq,
    Clear,
    Backspace,
    Neg,
    Open,
    Close,
    Fn(FnKind),
    Const(f64),
    Rand,
    Second,
    Angle,
    ProgOp(POp),
    PNot,
    Hex(char),
    Base(Base),
}

#[derive(Clone, Copy)]
enum HistVal {
    Real(f64),
    Int(i64),
}

struct HistEntry {
    expr: String,
    result: String,
    val: HistVal,
}

fn push_entry(history: &mut Vec<HistEntry>, expr: String, result: String, val: HistVal) {
    let mut expr = expr;
    while expr.ends_with('=') || expr.ends_with(' ') {
        expr.pop();
    }
    if expr.is_empty() {
        return;
    }
    history.insert(0, HistEntry { expr, result, val });
    if history.len() > 50 {
        history.truncate(50);
    }
}

struct CalcApp {
    mode: Mode,
    engine: Engine,
    prog: Programmer,
    rng: u64,
    history: Vec<HistEntry>,
    history_on: bool,
}

impl CalcApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.style_mut_of(egui::Theme::Dark, |style| {
            style.spacing.item_spacing = egui::vec2(GAP, GAP);
        });
        CalcApp {
            mode: Mode::Basic,
            engine: Engine::new(),
            prog: Programmer::new(),
            rng: 0x9E37_79B9_7F4A_7C15,
            history: Vec::new(),
            history_on: false,
        }
    }

    fn next_rand(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }

    fn set_mode(&mut self, mode: Mode, ctx: &egui::Context) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.resize(ctx);
    }

    fn resize(&self, ctx: &egui::Context) {
        let (w, h) = self.mode.size();
        let w = if self.history_on { w + HIST_W + GAP } else { w };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
    }

    fn insert_hist(&mut self, val: HistVal, ctx: &egui::Context) {
        match val {
            HistVal::Int(i) => {
                if matches!(self.mode, Mode::Programmer) {
                    self.prog.full_clear();
                    self.prog.value = i;
                    self.prog.fresh = false;
                } else {
                    self.engine.paste(&i.to_string());
                }
            }
            HistVal::Real(v) => {
                if matches!(self.mode, Mode::Programmer) {
                    let fits = v.fract() == 0.0 && v >= i64::MIN as f64 && v <= i64::MAX as f64;
                    if fits {
                        self.prog.full_clear();
                        self.prog.value = v as i64;
                        self.prog.fresh = false;
                    } else {
                        self.set_mode(Mode::Basic, ctx);
                        self.engine.paste(&format_f64(v));
                    }
                } else {
                    self.engine.paste(&format_f64(v));
                }
            }
        }
    }

    fn dispatch(&mut self, act: Act) {
        if matches!(self.mode, Mode::Programmer) {
            match act {
                Act::Digit(c) | Act::Hex(c) => self.prog.digit(c),
                Act::Dot => {}
                Act::Op(op) => {
                    let p = match op {
                        Op::Add => Some(POp::Add),
                        Op::Sub => Some(POp::Sub),
                        Op::Mul => Some(POp::Mul),
                        Op::Div => Some(POp::Div),
                        _ => None,
                    };
                    if let Some(p) = p {
                        self.prog.press_op(p);
                    }
                }
                Act::Percent => self.prog.percent(),
                Act::Eq => {
                    let expr = self.prog.expression_string();
                    if let Some(v) = self.prog.equals() {
                        let result = fmt_int(v, self.prog.base);
                        push_entry(&mut self.history, expr, result, HistVal::Int(v));
                    }
                }
                Act::Clear => self.prog.clear_press(),
                Act::Backspace => self.prog.backspace(),
                Act::Neg => self.prog.negate(),
                Act::ProgOp(p) => self.prog.press_op(p),
                Act::PNot => self.prog.not(),
                Act::Base(b) => self.prog.set_base(b),
                Act::Open
                | Act::Close
                | Act::Fn(_)
                | Act::Const(_)
                | Act::Rand
                | Act::Second
                | Act::Angle => {}
            }
            return;
        }
        match act {
            Act::Digit(c) => self.engine.digit(c),
            Act::Dot => self.engine.dot(),
            Act::Op(op) => self.engine.press_op(op),
            Act::Percent => self.engine.percent(),
            Act::Eq => {
                let expr = self.engine.expression_string();
                if let Some(v) = self.engine.equals() {
                    let result = format_f64(v);
                    push_entry(&mut self.history, expr, result, HistVal::Real(v));
                }
            }
            Act::Clear => self.engine.clear_press(),
            Act::Backspace => self.engine.backspace(),
            Act::Neg => self.engine.negate(),
            Act::Open => self.engine.open_paren(),
            Act::Close => self.engine.close_paren(),
            Act::Fn(k) => self.engine.unary(k),
            Act::Const(v) => self.engine.constant(v),
            Act::Rand => {
                let v = self.next_rand();
                self.engine.constant(v);
            }
            Act::Second => self.engine.second = !self.engine.second,
            Act::Angle => self.engine.angle_deg = !self.engine.angle_deg,
            Act::ProgOp(_) | Act::PNot | Act::Hex(_) | Act::Base(_) => {}
        }
    }

    fn act_for_char(&self, c: char) -> Option<Act> {
        match c {
            '0'..='9' => Some(Act::Digit(c)),
            '.' => Some(Act::Dot),
            '+' => Some(Act::Op(Op::Add)),
            '-' => Some(Act::Op(Op::Sub)),
            '*' => Some(Act::Op(Op::Mul)),
            '/' => Some(Act::Op(Op::Div)),
            '%' => Some(Act::Percent),
            '=' => Some(Act::Eq),
            '(' if self.mode == Mode::Scientific => Some(Act::Open),
            ')' if self.mode == Mode::Scientific => Some(Act::Close),
            'a'..='f' if self.mode == Mode::Programmer => Some(Act::Hex(c.to_ascii_uppercase())),
            'A'..='F' if self.mode == Mode::Programmer => Some(Act::Hex(c)),
            _ => None,
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let mut chars: Vec<char> = Vec::new();
        let mut keys: Vec<Act> = Vec::new();
        let mut paste: Option<String> = None;
        let mut copy = false;
        ctx.input(|i| {
            for e in &i.events {
                match e {
                    egui::Event::Copy => copy = true,
                    egui::Event::Paste(t) => paste = Some(t.clone()),
                    egui::Event::Text(t) => {
                        if i.modifiers.command || i.modifiers.ctrl {
                            continue;
                        }
                        chars.extend(t.chars());
                    }
                    egui::Event::Key {
                        key, pressed: true, ..
                    } => match key {
                        egui::Key::Enter => keys.push(Act::Eq),
                        egui::Key::Backspace => keys.push(Act::Backspace),
                        egui::Key::Escape => keys.push(Act::Clear),
                        _ => {}
                    },
                    _ => {}
                }
            }
        });
        if copy {
            let text = if matches!(self.mode, Mode::Programmer) {
                self.prog.copy_text()
            } else {
                self.engine.copy_text()
            };
            ctx.copy_text(text);
        }
        if let Some(t) = paste {
            if matches!(self.mode, Mode::Programmer) {
                self.prog.paste(&t);
            } else {
                self.engine.paste(&t);
            }
        }
        for c in chars {
            if let Some(act) = self.act_for_char(c) {
                self.dispatch(act);
            }
        }
        for act in keys {
            self.dispatch(act);
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        egui::containers::menu::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("View", |ui| {
                for (m, label) in [
                    (Mode::Basic, "Basic"),
                    (Mode::Scientific, "Scientific"),
                    (Mode::Programmer, "Programmer"),
                ] {
                    if ui.selectable_label(self.mode == m, label).clicked() {
                        self.set_mode(m, ui.ctx());
                        ui.close();
                    }
                }
                ui.separator();
                if ui.selectable_label(self.history_on, "History").clicked() {
                    self.history_on = !self.history_on;
                    self.resize(ui.ctx());
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Copy").clicked() {
                    let text = if matches!(self.mode, Mode::Programmer) {
                        self.prog.copy_text()
                    } else {
                        self.engine.copy_text()
                    };
                    ui.ctx().copy_text(text);
                    ui.close();
                }
                if ui.button("Clear").clicked() {
                    self.engine.full_clear();
                    self.prog.full_clear();
                    ui.close();
                }
            });
        });
    }

    fn paint_display(&self, ui: &mut egui::Ui, text: &str, max: f32, color: Color32, min_h: f32) {
        let avail = ui.available_width();
        let mut size = max;
        let galley = loop {
            let font = egui::FontId::proportional(size);
            let g = ui.painter().layout_no_wrap(text.to_string(), font, color);
            if g.size().x <= avail || size <= 16.0 {
                break g;
            }
            size -= 4.0;
        };
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(avail, galley.size().y.max(min_h)),
            egui::Sense::hover(),
        );
        let pos = egui::pos2(rect.right(), rect.center().y);
        ui.painter().text(
            pos,
            egui::Align2::RIGHT_CENTER,
            text,
            egui::FontId::proportional(size),
            color,
        );
    }

    fn draw_display(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        match self.mode {
            Mode::Basic | Mode::Scientific => {
                let expr = self.engine.expression_string();
                self.paint_display(ui, &expr, 17.0, DIM, 22.0);
                let text = self.engine.display_string();
                self.paint_display(ui, &text, 52.0, Color32::WHITE, 40.0);
            }
            Mode::Programmer => {
                let expr = self.prog.expression_string();
                self.paint_display(ui, &expr, 17.0, DIM, 22.0);
                let text = self.prog.display_string();
                self.paint_display(ui, &text, 44.0, Color32::WHITE, 40.0);
                let lines = self.prog.secondary_lines();
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    for line in lines {
                        ui.label(egui::RichText::new(line).size(13.0).color(DIM));
                    }
                });
            }
        }
        ui.add_space(GAP);
    }

    fn draw_history(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("History")
                    .size(15.0)
                    .color(Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    self.history.clear();
                }
            });
        });
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.history.is_empty() {
                    ui.label(
                        egui::RichText::new("No calculations yet")
                            .size(13.0)
                            .color(DIM),
                    );
                }
                for (i, e) in self.history.iter().enumerate() {
                    let w = ui.available_width();
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(w, 46.0), egui::Sense::click());
                    if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), SEG_BG);
                    }
                    ui.painter().text(
                        rect.left_top() + egui::vec2(8.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        e.expr.as_str(),
                        egui::FontId::proportional(12.0),
                        DIM,
                    );
                    let line = format!("= {}", e.result);
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(8.0, -6.0),
                        egui::Align2::LEFT_BOTTOM,
                        line.as_str(),
                        egui::FontId::proportional(16.0),
                        Color32::WHITE,
                    );
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            });
        if let Some(i) = clicked {
            let val = self.history[i].val;
            let ctx = ui.ctx().clone();
            self.insert_hist(val, &ctx);
        }
    }

    fn draw_body(&mut self, ui: &mut egui::Ui) {
        if !self.history_on {
            self.draw_display(ui);
            self.draw_pads(ui);
            return;
        }
        let h = ui.available_height();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let left_w = (ui.available_width() - HIST_W - GAP).max(120.0);
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.draw_display(ui);
                    self.draw_pads(ui);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(HIST_W, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.draw_history(ui),
            );
        });
    }

    fn basic_rows(&self) -> Vec<Vec<Cell>> {
        let clear = if matches!(self.mode, Mode::Programmer) {
            self.prog.clear_label()
        } else {
            self.engine.clear_label()
        };
        let last_row: Vec<Cell> = if matches!(self.mode, Mode::Programmer) {
            vec![
                (2.0, "0", Kind::Digit, 24.0, Act::Digit('0')),
                (2.0, "=", Kind::Eq, 28.0, Act::Eq),
            ]
        } else {
            vec![
                (2.0, "0", Kind::Digit, 24.0, Act::Digit('0')),
                (1.0, ".", Kind::Digit, 24.0, Act::Dot),
                (1.0, "=", Kind::Eq, 28.0, Act::Eq),
            ]
        };
        vec![
            vec![
                (1.0, clear, Kind::Top, 19.0, Act::Clear),
                (1.0, "+/−", Kind::Top, 20.0, Act::Neg),
                (1.0, "%", Kind::Top, 22.0, Act::Percent),
                (1.0, "÷", Kind::Op, 28.0, Act::Op(Op::Div)),
            ],
            vec![
                (1.0, "7", Kind::Digit, 24.0, Act::Digit('7')),
                (1.0, "8", Kind::Digit, 24.0, Act::Digit('8')),
                (1.0, "9", Kind::Digit, 24.0, Act::Digit('9')),
                (1.0, "×", Kind::Op, 28.0, Act::Op(Op::Mul)),
            ],
            vec![
                (1.0, "4", Kind::Digit, 24.0, Act::Digit('4')),
                (1.0, "5", Kind::Digit, 24.0, Act::Digit('5')),
                (1.0, "6", Kind::Digit, 24.0, Act::Digit('6')),
                (1.0, "−", Kind::Op, 28.0, Act::Op(Op::Sub)),
            ],
            vec![
                (1.0, "1", Kind::Digit, 24.0, Act::Digit('1')),
                (1.0, "2", Kind::Digit, 24.0, Act::Digit('2')),
                (1.0, "3", Kind::Digit, 24.0, Act::Digit('3')),
                (1.0, "+", Kind::Op, 28.0, Act::Op(Op::Add)),
            ],
            last_row,
        ]
    }

    fn sci_fn_rows(&self) -> Vec<Vec<Cell>> {
        let second = self.engine.second;
        let trig = |k: FnKind, label: &'static str, alt: &'static str| -> Cell {
            let kind = if second { Kind::FnHot } else { Kind::Fn };
            let text = if second { alt } else { label };
            let size = if second { 14.0 } else { 17.0 };
            (1.0, text, kind, size, Act::Fn(k))
        };
        let angle_label = if self.engine.angle_deg { "DEG" } else { "RAD" };
        let angle_kind = if self.engine.angle_deg {
            Kind::Fn
        } else {
            Kind::FnHot
        };
        vec![
            vec![
                (
                    1.0,
                    "2nd",
                    if second { Kind::FnHot } else { Kind::Fn },
                    16.0,
                    Act::Second,
                ),
                (1.0, "(", Kind::Fn, 22.0, Act::Open),
                (1.0, ")", Kind::Fn, 22.0, Act::Close),
                (1.0, angle_label, angle_kind, 15.0, Act::Angle),
            ],
            vec![
                trig(FnKind::Sin, "sin", "sin⁻¹"),
                trig(FnKind::Cos, "cos", "cos⁻¹"),
                trig(FnKind::Tan, "tan", "tan⁻¹"),
                (1.0, "x!", Kind::Fn, 18.0, Act::Fn(FnKind::Fact)),
            ],
            vec![
                (1.0, "ln", Kind::Fn, 18.0, Act::Fn(FnKind::Ln)),
                (1.0, "log", Kind::Fn, 17.0, Act::Fn(FnKind::Log)),
                (1.0, "x²", Kind::Fn, 18.0, Act::Fn(FnKind::Square)),
                (1.0, "xʸ", Kind::Fn, 18.0, Act::Op(Op::Pow)),
            ],
            vec![
                (1.0, "√", Kind::Fn, 20.0, Act::Fn(FnKind::Sqrt)),
                (1.0, "1/x", Kind::Fn, 17.0, Act::Fn(FnKind::Inv)),
                (1.0, "eˣ", Kind::Fn, 18.0, Act::Fn(FnKind::Exp)),
                (1.0, "10ˣ", Kind::Fn, 16.0, Act::Fn(FnKind::Exp10)),
            ],
            vec![
                (1.0, "π", Kind::Fn, 20.0, Act::Const(std::f64::consts::PI)),
                (1.0, "e", Kind::Fn, 20.0, Act::Const(std::f64::consts::E)),
                (1.0, "mod", Kind::Fn, 16.0, Act::Op(Op::Mod)),
                (1.0, "Rand", Kind::Fn, 15.0, Act::Rand),
            ],
        ]
    }

    fn prog_fn_rows(&self) -> Vec<Vec<Cell>> {
        let hex_on = self.prog.base == Base::Hex;
        let hex_kind = if hex_on { Kind::Fn } else { Kind::Disabled };
        vec![
            vec![
                (1.0, "NOT", Kind::Fn, 16.0, Act::PNot),
                (1.0, "AND", Kind::Fn, 15.0, Act::ProgOp(POp::And)),
                (1.0, "XOR", Kind::Fn, 15.0, Act::ProgOp(POp::Xor)),
                (1.0, "OR", Kind::Fn, 15.0, Act::ProgOp(POp::Or)),
            ],
            vec![
                (1.0, "Lsh", Kind::Fn, 16.0, Act::ProgOp(POp::Shl)),
                (1.0, "Rsh", Kind::Fn, 16.0, Act::ProgOp(POp::Shr)),
                (1.0, "A", hex_kind, 20.0, Act::Hex('A')),
                (1.0, "B", hex_kind, 20.0, Act::Hex('B')),
            ],
            vec![
                (1.0, "C", hex_kind, 20.0, Act::Hex('C')),
                (1.0, "D", hex_kind, 20.0, Act::Hex('D')),
                (1.0, "E", hex_kind, 20.0, Act::Hex('E')),
                (1.0, "F", hex_kind, 20.0, Act::Hex('F')),
            ],
        ]
    }

    fn seg_cells(&self) -> Vec<Cell> {
        let seg = |label: &'static str, base: Base| -> Cell {
            let kind = if self.prog.base == base {
                Kind::SegOn
            } else {
                Kind::Seg
            };
            (1.0, label, kind, 15.0, Act::Base(base))
        };
        vec![
            seg("HEX", Base::Hex),
            seg("OCT", Base::Oct),
            seg("DEC", Base::Dec),
            seg("BIN", Base::Bin),
        ]
    }

    fn draw_pads(&mut self, ui: &mut egui::Ui) {
        let basic = self.basic_rows();
        let mut clicked = None;
        match self.mode {
            Mode::Basic => {
                for row in &basic {
                    let w = ui.available_width();
                    if let Some(act) = draw_row(ui, w, row, ROW_H) {
                        clicked = Some(act);
                    }
                }
            }
            Mode::Scientific => {
                let fns = self.sci_fn_rows();
                ui.columns(2, |cols| {
                    let w0 = cols[0].available_width();
                    let w1 = cols[1].available_width();
                    if let Some(act) = draw_column(&mut cols[0], w0, &fns, ROW_H) {
                        clicked = Some(act);
                    }
                    if let Some(act) = draw_column(&mut cols[1], w1, &basic, ROW_H) {
                        clicked = Some(act);
                    }
                });
            }
            Mode::Programmer => {
                let fns = self.prog_fn_rows();
                ui.columns(2, |cols| {
                    let w0 = cols[0].available_width();
                    let w1 = cols[1].available_width();
                    if let Some(act) = draw_column(&mut cols[0], w0, &fns, ROW_H) {
                        clicked = Some(act);
                    }
                    if let Some(act) = draw_column(&mut cols[1], w1, &basic, ROW_H) {
                        clicked = Some(act);
                    }
                });
            }
        }
        if let Some(act) = clicked.take() {
            self.dispatch(act);
        }
        if self.mode == Mode::Programmer {
            let seg = self.seg_cells();
            ui.add_space(GAP);
            let w = ui.available_width();
            if let Some(act) = draw_row(ui, w, &seg, 32.0) {
                self.dispatch(act);
            }
        }
    }
}

impl eframe::App for CalcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().memory_mut(|mem| {
            if let Some(id) = mem.focused() {
                mem.surrender_focus(id);
            }
        });
        self.handle_keys(ui.ctx());
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BLACK)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ui, |ui| {
                self.menu(ui);
                self.draw_body(ui);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Engine {
        fn run_act(&mut self, act: Act) {
            match act {
                Act::Digit(c) => self.digit(c),
                Act::Dot => self.dot(),
                Act::Op(op) => self.press_op(op),
                Act::Percent => self.percent(),
                Act::Eq => {
                    let _ = self.equals();
                }
                Act::Clear => self.clear_press(),
                Act::Backspace => self.backspace(),
                Act::Neg => self.negate(),
                Act::Open => self.open_paren(),
                Act::Close => self.close_paren(),
                Act::Fn(k) => self.unary(k),
                Act::Const(v) => self.constant(v),
                Act::Rand => {}
                Act::Second => self.second = !self.second,
                Act::Angle => self.angle_deg = !self.angle_deg,
                Act::ProgOp(_) | Act::PNot | Act::Hex(_) | Act::Base(_) => {}
            }
        }
    }

    fn run(acts: &[Act]) -> Engine {
        let mut e = Engine::new();
        for a in acts {
            e.run_act(*a);
        }
        e
    }

    fn digits(s: &str) -> Vec<Act> {
        s.chars().map(Act::Digit).collect()
    }

    #[test]
    fn precedence() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("3"));
        acts.push(Act::Op(Op::Mul));
        acts.extend(digits("4"));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "24");
    }

    #[test]
    fn chain_folds_running_total() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("34"));
        acts.push(Act::Op(Op::Sub));
        acts.extend(digits("5"));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "41");
    }

    #[test]
    fn percent_addition() {
        let mut acts = digits("200");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("10"));
        acts.push(Act::Percent);
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "220");
    }

    #[test]
    fn percent_multiplication() {
        let mut acts = digits("200");
        acts.push(Act::Op(Op::Mul));
        acts.extend(digits("10"));
        acts.push(Act::Percent);
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "20");
    }

    #[test]
    fn repeat_equals() {
        let mut acts = digits("1");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("1"));
        acts.push(Act::Eq);
        let mut e = run(&acts);
        assert_eq!(e.display_string(), "2");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "3");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "4");
    }

    #[test]
    fn divide_by_zero_shows_error() {
        let mut acts = digits("1");
        acts.push(Act::Op(Op::Div));
        acts.extend(digits("0"));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "Error");
    }

    #[test]
    fn float_cleanup() {
        let mut acts = digits("1");
        acts.push(Act::Dot);
        acts.extend(digits("1"));
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("2"));
        acts.push(Act::Dot);
        acts.extend(digits("2"));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "3.3");
    }

    #[test]
    fn grouping_while_typing_and_result() {
        let mut e = run(&digits("1234567"));
        assert_eq!(e.display_string(), "1,234,567");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "1,234,567");
    }

    #[test]
    fn negate_entry() {
        let mut e = run(&digits("5"));
        e.run_act(Act::Neg);
        assert_eq!(e.display_string(), "-5");
        e.run_act(Act::Neg);
        assert_eq!(e.display_string(), "5");
    }

    #[test]
    fn parentheses_precedence() {
        let mut acts = vec![Act::Open];
        acts.extend(digits("2"));
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("3"));
        acts.push(Act::Close);
        acts.push(Act::Op(Op::Mul));
        acts.extend(digits("4"));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "20");
    }

    #[test]
    fn factorial_and_sqrt() {
        let mut acts = digits("5");
        acts.push(Act::Fn(FnKind::Fact));
        assert_eq!(run(&acts).display_string(), "120");
        let mut acts = digits("9");
        acts.push(Act::Fn(FnKind::Sqrt));
        assert_eq!(run(&acts).display_string(), "3");
    }

    #[test]
    fn clear_keeps_pending_operand() {
        let mut acts = digits("200");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("10"));
        let mut e = run(&acts);
        assert_eq!(e.clear_label(), "C");
        e.run_act(Act::Clear);
        assert_eq!(e.display_string(), "0");
        for a in digits("5") {
            e.run_act(a);
        }
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "205");
    }

    #[test]
    fn ac_after_zero() {
        let mut e = run(&digits("0"));
        e.run_act(Act::Clear);
        assert_eq!(e.clear_label(), "AC");
        e.run_act(Act::Clear);
        assert_eq!(e.display_string(), "0");
        assert!(!e.error);
    }

    #[test]
    fn programmer_hex_xor() {
        let mut p = Programmer::new();
        p.set_base(Base::Hex);
        for c in "FF".chars() {
            p.digit(c);
        }
        assert_eq!(p.display_string(), "FF");
        p.press_op(POp::Xor);
        for c in "0F".chars() {
            p.digit(c);
        }
        p.equals();
        assert_eq!(p.display_string(), "F0");
    }

    #[test]
    fn programmer_base_switch() {
        let mut p = Programmer::new();
        for c in "10".chars() {
            p.digit(c);
        }
        p.set_base(Base::Hex);
        assert_eq!(p.display_string(), "A");
        p.set_base(Base::Bin);
        assert_eq!(p.display_string(), "1010");
    }

    #[test]
    fn programmer_div_by_zero() {
        let mut p = Programmer::new();
        p.digit('5');
        p.press_op(POp::Div);
        p.digit('0');
        p.equals();
        assert_eq!(p.display_string(), "Error");
    }

    #[test]
    fn programmer_not_and_shl() {
        let mut p = Programmer::new();
        p.digit('0');
        p.not();
        assert_eq!(p.display_string(), "-1");
        let mut p = Programmer::new();
        p.digit('1');
        p.press_op(POp::Shl);
        p.digit('4');
        p.equals();
        assert_eq!(p.display_string(), "16");
    }

    #[test]
    fn expression_shows_typed_expression() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("3"));
        let e = run(&acts);
        assert_eq!(e.expression_string(), "12 + 3");
    }

    #[test]
    fn expression_falls_back_to_last_equals() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("3"));
        acts.push(Act::Eq);
        let e = run(&acts);
        assert_eq!(e.display_string(), "15");
        assert_eq!(e.expression_string(), "12 + 3 =");
    }

    #[test]
    fn expression_hidden_for_bare_value() {
        let e = run(&digits("5"));
        assert_eq!(e.expression_string(), "");
        let e = run(&[Act::Dot]);
        assert_eq!(e.expression_string(), "");
    }

    #[test]
    fn expression_cleared_when_typing_new_number() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("3"));
        acts.push(Act::Eq);
        let mut e = run(&acts);
        e.run_act(Act::Digit('7'));
        assert_eq!(e.expression_string(), "");
    }

    #[test]
    fn expression_operator_pending() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Mul));
        let e = run(&acts);
        assert_eq!(e.expression_string(), "12 ×");
    }

    #[test]
    fn expression_inside_paren() {
        let mut acts = vec![Act::Open];
        acts.extend(digits("3"));
        let e = run(&acts);
        assert_eq!(e.expression_string(), "( 3");
    }

    #[test]
    fn op_after_equals_does_not_restart_calculation() {
        let mut acts = digits("1");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("1"));
        acts.push(Act::Eq);
        let mut e = run(&acts);
        assert_eq!(e.display_string(), "2");
        e.run_act(Act::Op(Op::Add));
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "4");
    }

    #[test]
    fn equals_returns_value_only_for_real_evaluations() {
        let mut acts = digits("1");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("1"));
        let mut e = run(&acts);
        assert_eq!(e.equals(), Some(2.0));
        assert_eq!(e.equals(), None);
        let mut e = run(&digits("7"));
        assert_eq!(e.equals(), None);
    }

    #[test]
    fn programmer_expression_and_equals() {
        let mut p = Programmer::new();
        p.digit('5');
        p.press_op(POp::Add);
        p.digit('3');
        assert_eq!(p.expression_string(), "5 + 3");
        assert_eq!(p.equals(), Some(8));
        assert_eq!(p.expression_string(), "5 + 3 =");
        assert_eq!(p.equals(), None);
    }

    #[test]
    fn history_push_strips_and_caps() {
        let mut h: Vec<HistEntry> = Vec::new();
        push_entry(
            &mut h,
            "12 + 3 =".to_string(),
            "15".to_string(),
            HistVal::Real(15.0),
        );
        push_entry(&mut h, "".to_string(), "7".to_string(), HistVal::Real(7.0));
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].expr, "12 + 3");
        for i in 0..60 {
            push_entry(
                &mut h,
                format!("{i} + 1 ="),
                "2".to_string(),
                HistVal::Real(2.0),
            );
        }
        assert_eq!(h.len(), 50);
        assert_eq!(h[0].expr, "59 + 1");
    }
}
