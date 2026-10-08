use eframe::egui::{self, Color32};

const GAP: f32 = 8.0;
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
const SCI_SIZE: (f32, f32) = (780.0, 560.0);
const PROG_SIZE: (f32, f32) = (660.0, 630.0);

const GOLDEN: f64 = 1.618_033_988_749_895;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([BASIC_SIZE.0, BASIC_SIZE.1])
            .with_min_inner_size([BASIC_SIZE.0, BASIC_SIZE.1])
            .with_resizable(true)
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

fn display_with_map(s: &str) -> (String, Vec<usize>) {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut map: Vec<usize> = vec![0];
    let mut pos = 0usize;
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0usize;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || (chars[i] == '.'
                        && dots == 0
                        && i + 1 < chars.len()
                        && chars[i + 1].is_ascii_digit()))
            {
                if chars[i] == '.' {
                    dots += 1;
                }
                i += 1;
            }
            let run: String = chars[start..i].iter().collect();
            for ch in group(&run).chars() {
                out.push(ch);
                if ch == ',' {
                    map.push(pos);
                } else {
                    pos += 1;
                    map.push(pos);
                }
            }
        } else {
            out.push(chars[i]);
            pos += 1;
            map.push(pos);
            i += 1;
        }
    }
    (out, map)
}

fn caret_display_index(map: &[usize], cursor: usize) -> usize {
    if map.is_empty() {
        return 0;
    }
    let mut d = map
        .iter()
        .position(|&m| m >= cursor)
        .unwrap_or(map.len() - 1);
    while d + 1 < map.len() && map[d + 1] == map[d] {
        d += 1;
    }
    d
}

fn click_cursor(map: &[usize], d: usize) -> usize {
    let i = d.min(map.len().saturating_sub(1));
    map.get(i).copied().unwrap_or(0)
}

fn is_plain_number(s: &str) -> bool {
    let mut t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.starts_with('-') || t.starts_with('\u{2212}') {
        t.remove(0);
    }
    !t.is_empty()
        && t.chars().all(|c| c.is_ascii_digit() || c == '.')
        && t.matches('.').count() <= 1
}

fn is_plain_zero(s: &str) -> bool {
    matches!(s.trim().parse::<f64>(), Ok(v) if v == 0.0)
}

fn last_bin_op_chars(c: &[char]) -> Option<(usize, usize)> {
    let n = c.len();
    let mut depth = 0i32;
    let mut last: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < n {
        let ch = c[i];
        if ch == '(' {
            depth += 1;
            i += 1;
            continue;
        }
        if ch == ')' {
            depth -= 1;
            i += 1;
            continue;
        }
        let mut advance = 1usize;
        if depth == 0 {
            let op_len = if matches!(ch, '+' | '\u{2212}' | '-' | '\u{d7}' | '\u{f7}' | '^') {
                Some(1usize)
            } else if (ch == 'm' || ch == 'M') && i + 3 <= n {
                let word: String = c[i..i + 3].iter().collect::<String>().to_ascii_lowercase();
                let after_ok = i + 3 == n || !c[i + 3].is_alphabetic();
                let before_ok = i == 0 || !c[i - 1].is_alphabetic();
                if word == "mod" && after_ok && before_ok {
                    Some(3)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(len) = op_len {
                let mut j = i;
                while j > 0 && c[j - 1].is_whitespace() {
                    j -= 1;
                }
                let binary = j > 0
                    && !matches!(
                        c[j - 1],
                        '+' | '\u{2212}' | '-' | '\u{d7}' | '\u{f7}' | '^' | '(' | ','
                    );
                if binary {
                    last = Some((i, i + len));
                }
                advance = len;
            }
        }
        i += advance;
    }
    last
}

fn balance_parens(s: &str) -> String {
    let opens = s.matches('(').count();
    let closes = s.matches(')').count();
    let mut out = s.to_string();
    if opens > closes {
        out.push_str(&")".repeat(opens - closes));
    }
    out
}

fn has_open_func(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth > 0
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Bang,
    LParen,
    RParen,
    Comma,
    Mod,
    Pow2,
    Pow3,
}

const FN_NAMES: &[&str] = &[
    "sin⁻¹",
    "cos⁻¹",
    "tan⁻¹",
    "csc⁻¹",
    "sec⁻¹",
    "cot⁻¹",
    "cosec⁻¹",
    "cosec",
    "csc",
    "sec",
    "cot",
    "asin",
    "acos",
    "atan",
    "sin",
    "cos",
    "tan",
    "floor",
    "ceil",
    "round",
    "cbrt",
    "sign",
    "ncr",
    "npr",
    "gcd",
    "lcm",
    "min",
    "max",
    "phi",
    "log2",
    "ln",
    "log",
    "sqrt",
    "exp",
    "abs",
    "√",
    "pi",
    "π",
];

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text: String = chars[start..i].iter().collect();
            let v: f64 = if text == "." {
                0.0
            } else {
                text.parse().map_err(|_| format!("bad number '{}'", text))?
            };
            out.push(Tok::Num(v));
            continue;
        }
        if c.is_alphabetic() {
            let rest: String = chars[i..].iter().collect();
            let rest_l = rest.to_ascii_lowercase();
            if rest_l.starts_with("mod") {
                i += 3;
                out.push(Tok::Mod);
                continue;
            }
            if let Some(name) = FN_NAMES.iter().find(|n| rest_l.starts_with(**n)) {
                i += name.chars().count();
                out.push(Tok::Ident(name.to_string()));
                continue;
            }
            out.push(Tok::Ident(c.to_ascii_lowercase().to_string()));
            i += 1;
            continue;
        }
        match c {
            '+' => out.push(Tok::Plus),
            '-' | '\u{2212}' => out.push(Tok::Minus),
            '*' | '\u{d7}' => out.push(Tok::Star),
            '/' | '\u{f7}' => out.push(Tok::Slash),
            '^' => out.push(Tok::Caret),
            '!' => out.push(Tok::Bang),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            ',' => out.push(Tok::Comma),
            '√' => out.push(Tok::Ident("√".to_string())),
            '\u{b2}' => out.push(Tok::Pow2),
            '\u{b3}' => out.push(Tok::Pow3),
            _ => return Err(format!("unexpected character '{}'", c)),
        }
        i += 1;
    }
    Ok(out)
}

fn factorial_val(x: f64) -> Result<f64, String> {
    if x < 0.0 || x.fract() != 0.0 || x > 170.0 {
        return Err("factorial requires a whole number from 0 to 170".to_string());
    }
    let mut acc = 1.0;
    for i in 2..=(x as i64) {
        acc *= i as f64;
    }
    Ok(acc)
}

struct PExpr<'a> {
    toks: &'a [Tok],
    pos: usize,
    deg: bool,
    vars: &'a [(char, f64)],
    lenient: bool,
}

impl<'a> PExpr<'a> {
    fn peek(&self) -> Option<&'a Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        loop {
            if self.eat(&Tok::Plus) {
                v += self.term()?;
            } else if self.eat(&Tok::Minus) {
                v -= self.term()?;
            } else {
                return Ok(v);
            }
        }
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.unary()?;
        loop {
            if self.eat(&Tok::Star) {
                v *= self.unary()?;
            } else if self.eat(&Tok::Slash) {
                let d = self.unary()?;
                if d == 0.0 {
                    return Err("division by zero".to_string());
                }
                v /= d;
            } else if self.eat(&Tok::Mod) {
                let d = self.unary()?;
                if d == 0.0 {
                    return Err("modulo by zero".to_string());
                }
                v %= d;
            } else {
                match self.peek() {
                    Some(Tok::Num(_)) | Some(Tok::Ident(_)) | Some(Tok::LParen) => {
                        v *= self.unary()?;
                    }
                    _ => return Ok(v),
                }
            }
        }
    }

    fn unary(&mut self) -> Result<f64, String> {
        if self.eat(&Tok::Minus) {
            return Ok(-self.unary()?);
        }
        if self.eat(&Tok::Plus) {
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> Result<f64, String> {
        let base = self.postfix()?;
        if self.eat(&Tok::Caret) {
            let e = self.unary()?;
            return Ok(base.powf(e));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<f64, String> {
        let mut v = self.primary()?;
        loop {
            if self.eat(&Tok::Bang) {
                v = factorial_val(v)?;
            } else if self.eat(&Tok::Pow2) {
                v *= v;
            } else if self.eat(&Tok::Pow3) {
                v = v * v * v;
            } else {
                break;
            }
        }
        Ok(v)
    }

    fn primary(&mut self) -> Result<f64, String> {
        let Some(tok) = self.peek().cloned() else {
            if self.lenient {
                return Ok(0.0);
            }
            return Err("unexpected end of expression".to_string());
        };
        match tok {
            Tok::Num(v) => {
                self.pos += 1;
                Ok(v)
            }
            Tok::LParen => {
                self.pos += 1;
                if self.eat(&Tok::RParen) {
                    return Ok(0.0);
                }
                let v = self.expr()?;
                self.eat(&Tok::RParen);
                Ok(v)
            }
            Tok::Ident(name) => {
                self.pos += 1;
                self.ident(&name)
            }
            _ => Err("expected a value".to_string()),
        }
    }

    fn ident(&mut self, name: &str) -> Result<f64, String> {
        match name {
            "pi" | "π" => return Ok(std::f64::consts::PI),
            "e" => return Ok(std::f64::consts::E),
            "phi" | "φ" => return Ok(GOLDEN),
            _ => {}
        }
        if FN_NAMES.contains(&name) {
            let mut args: Vec<f64> = Vec::new();
            if self.peek() == Some(&Tok::LParen) {
                self.pos += 1;
                if self.eat(&Tok::RParen) {
                    args.push(0.0);
                } else {
                    args.push(self.expr()?);
                    while self.eat(&Tok::Comma) {
                        args.push(self.expr()?);
                    }
                    self.eat(&Tok::RParen);
                }
            } else {
                args.push(self.unary()?);
            }
            return apply_fn_args(name, &args, self.deg);
        }
        if name.chars().count() == 1 {
            let c = name.chars().next().unwrap();
            if let Some((_, v)) = self.vars.iter().find(|(k, _)| *k == c) {
                return Ok(*v);
            }
            return Err(format!("unknown variable '{}'", name));
        }
        Err(format!("unknown function '{}'", name))
    }
}

fn apply_fn(name: &str, x: f64, deg: bool) -> Result<f64, String> {
    let r = match name {
        "sin" | "cos" | "tan" => {
            let a = if deg { x.to_radians() } else { x };
            match name {
                "sin" => a.sin(),
                "cos" => a.cos(),
                _ => a.tan(),
            }
        }
        "csc" | "cosec" => {
            let a = if deg { x.to_radians() } else { x };
            1.0 / a.sin()
        }
        "sec" => {
            let a = if deg { x.to_radians() } else { x };
            1.0 / a.cos()
        }
        "cot" => {
            let a = if deg { x.to_radians() } else { x };
            1.0 / a.tan()
        }
        "asin" | "acos" | "atan" | "sin⁻¹" | "cos⁻¹" | "tan⁻¹" => {
            let v = match name {
                "asin" | "sin⁻¹" => x.asin(),
                "acos" | "cos⁻¹" => x.acos(),
                _ => x.atan(),
            };
            if deg { v.to_degrees() } else { v }
        }
        "csc⁻¹" | "cosec⁻¹" => {
            let v = (1.0 / x).asin();
            if deg { v.to_degrees() } else { v }
        }
        "sec⁻¹" => {
            let v = (1.0 / x).acos();
            if deg { v.to_degrees() } else { v }
        }
        "cot⁻¹" => {
            let v = (1.0 / x).atan();
            if deg { v.to_degrees() } else { v }
        }
        "ln" => x.ln(),
        "log" => x.log10(),
        "log2" => x.log2(),
        "sqrt" | "√" => x.sqrt(),
        "cbrt" => x.cbrt(),
        "exp" => x.exp(),
        "abs" => x.abs(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sign" => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
        _ => return Err(format!("unknown function '{}'", name)),
    };
    Ok(r)
}

fn int_whole(x: f64) -> Result<i64, String> {
    if x.fract() != 0.0 || x.abs() >= 1e15 {
        return Err("expected a whole number".to_string());
    }
    Ok(x as i64)
}

fn combination(n: f64, r: f64, perm: bool) -> Result<f64, String> {
    if n < 0.0 || r < 0.0 || n.fract() != 0.0 || r.fract() != 0.0 || r > n || n > 170.0 {
        return Err("nCr/nPr need whole numbers with 0 <= r <= n <= 170".to_string());
    }
    let n = n as i64;
    let r = r as i64;
    if perm {
        let mut acc = 1.0;
        for i in 0..r {
            acc *= (n - i) as f64;
        }
        return Ok(acc);
    }
    let r = r.min(n - r);
    let mut acc = 1.0;
    for i in 1..=r {
        acc = acc * (n - r + i) as f64 / i as f64;
    }
    Ok(acc)
}

fn fold_gcd(args: &[f64]) -> Result<f64, String> {
    let mut g = int_whole(args[0])?.unsigned_abs();
    for &a in &args[1..] {
        let mut b = int_whole(a)?.unsigned_abs();
        while b != 0 {
            let t = g % b;
            g = b;
            b = t;
        }
    }
    Ok(g as f64)
}

fn fold_lcm(args: &[f64]) -> Result<f64, String> {
    let mut l = int_whole(args[0])?.unsigned_abs();
    for &a in &args[1..] {
        let b = int_whole(a)?.unsigned_abs();
        let (mut x, mut y) = (l, b);
        while y != 0 {
            let t = x % y;
            x = y;
            y = t;
        }
        l = l.checked_div(x).map_or(0, |q| q.saturating_mul(b));
    }
    Ok(l as f64)
}

fn apply_fn_args(name: &str, args: &[f64], deg: bool) -> Result<f64, String> {
    match name {
        "ncr" | "npr" => {
            if args.len() != 2 {
                return Err(format!("{} needs two arguments", name));
            }
            combination(args[0], args[1], name == "npr")
        }
        "gcd" => {
            if args.len() < 2 {
                return Err("gcd needs at least two arguments".to_string());
            }
            fold_gcd(args)
        }
        "lcm" => {
            if args.len() < 2 {
                return Err("lcm needs at least two arguments".to_string());
            }
            fold_lcm(args)
        }
        "min" | "max" => {
            if args.len() < 2 {
                return Err(format!("{} needs at least two arguments", name));
            }
            let mut acc = args[0];
            for &a in &args[1..] {
                acc = if name == "min" {
                    acc.min(a)
                } else {
                    acc.max(a)
                };
            }
            Ok(acc)
        }
        _ => {
            if args.len() != 1 {
                return Err(format!("{} needs one argument", name));
            }
            apply_fn(name, args[0], deg)
        }
    }
}

fn eval_str(s: &str, deg: bool, vars: &[(char, f64)], lenient: bool) -> Result<f64, String> {
    if s.trim().is_empty() {
        return Err("empty expression".to_string());
    }
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return Err("empty expression".to_string());
    }
    let mut p = PExpr {
        toks: &toks,
        pos: 0,
        deg,
        vars,
        lenient,
    };
    let v = p.expr()?;
    if p.pos != toks.len() {
        return Err("unexpected trailing input".to_string());
    }
    Ok(v)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FnKind {
    Sin,
    Cos,
    Tan,
    Csc,
    Sec,
    Cot,
    Ln,
    Log,
    Sqrt,
    Inv,
    Square,
    Cube,
    Exp,
    Exp10,
    Fact,
}

struct Engine {
    entry: String,
    cursor: usize,
    evaluated: bool,
    error: bool,
    last_expr: String,
    last_op: Option<(String, String)>,
    angle_deg: bool,
    second: bool,
    result: f64,
    has_result: bool,
}

impl Engine {
    fn new() -> Self {
        Engine {
            entry: String::new(),
            cursor: 0,
            evaluated: false,
            error: false,
            last_expr: String::new(),
            last_op: None,
            angle_deg: true,
            second: false,
            result: 0.0,
            has_result: false,
        }
    }

    fn chars_len(&self) -> usize {
        self.entry.chars().count()
    }

    fn byte_at(&self, ci: usize) -> usize {
        self.entry
            .char_indices()
            .nth(ci)
            .map(|(b, _)| b)
            .unwrap_or(self.entry.len())
    }

    fn slice(&self, s: usize, e: usize) -> String {
        let a = self.byte_at(s);
        let b = self.byte_at(e);
        self.entry[a..b].to_string()
    }

    fn replace_span(&mut self, s: usize, e: usize, text: &str) {
        let a = self.byte_at(s);
        let b = self.byte_at(e);
        self.entry.replace_range(a..b, text);
        self.cursor = s + text.chars().count();
    }

    fn insert_at(&mut self, ci: usize, text: &str) {
        let b = self.byte_at(ci);
        self.entry.insert_str(b, text);
    }

    fn match_close(c: &[char], open: usize) -> Option<usize> {
        let mut depth = 0i32;
        for (i, ch) in c.iter().enumerate().skip(open) {
            if *ch == '(' {
                depth += 1;
            } else if *ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    fn match_open(c: &[char], close: usize) -> Option<usize> {
        let mut depth = 0i32;
        for i in (0..=close).rev() {
            if c[i] == ')' {
                depth += 1;
            } else if c[i] == '(' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    fn forward_operand_end(&self, from: usize) -> usize {
        let c: Vec<char> = self.entry.chars().collect();
        let n = c.len();
        let mut i = from.min(n);
        loop {
            if i >= n {
                break;
            }
            let ch = c[i];
            if ch.is_ascii_digit() || ch == '.' {
                let before = i;
                let mut dots = 0usize;
                while i < n
                    && (c[i].is_ascii_digit()
                        || (c[i] == '.' && dots == 0 && i + 1 < n && c[i + 1].is_ascii_digit()))
                {
                    if c[i] == '.' {
                        dots += 1;
                    }
                    i += 1;
                }
                while i < n && (c[i] == '\u{b2}' || c[i] == '\u{b3}' || c[i] == '!') {
                    i += 1;
                }
                if i == before {
                    break;
                }
            } else if ch == '\u{b2}' || ch == '\u{b3}' || ch == '!' {
                while i < n && (c[i] == '\u{b2}' || c[i] == '\u{b3}' || c[i] == '!') {
                    i += 1;
                }
            } else if ch == '(' {
                i = Self::match_close(&c, i).map(|x| x + 1).unwrap_or(n);
            } else if ch.is_alphabetic() || ch == '\u{221a}' || ch == '\u{207b}' || ch == '\u{b9}' {
                while i < n
                    && (c[i].is_alphabetic()
                        || c[i] == '\u{221a}'
                        || c[i] == '\u{207b}'
                        || c[i] == '\u{b9}')
                {
                    i += 1;
                }
                if i < n && c[i] == '(' {
                    i = Self::match_close(&c, i).map(|x| x + 1).unwrap_or(n);
                }
            } else {
                break;
            }
        }
        i
    }

    fn operand_span(&self, ci: usize) -> (usize, usize) {
        let c: Vec<char> = self.entry.chars().collect();
        let n = c.len();
        let ci = ci.min(n);
        let mut s = ci;
        if ci > 0 {
            let p = c[ci - 1];
            let touch = p.is_ascii_digit()
                || p == '.'
                || p == ')'
                || p == '\u{b2}'
                || p == '\u{b3}'
                || p == '!'
                || p == '\u{3c0}'
                || p == 'e';
            if touch {
                let mut q = ci;
                while q > 0 && (c[q - 1] == '\u{b2}' || c[q - 1] == '\u{b3}' || c[q - 1] == '!') {
                    q -= 1;
                }
                if q > 0 && c[q - 1] == ')' {
                    let open = Self::match_open(&c, q - 1).unwrap_or(q - 1);
                    s = open;
                    while s > 0 {
                        let b = c[s - 1];
                        if b.is_alphabetic()
                            || b == '\u{221a}'
                            || b == '\u{b2}'
                            || b == '\u{b3}'
                            || b == '!'
                            || b.is_ascii_digit()
                            || b == '.'
                        {
                            s -= 1;
                        } else {
                            break;
                        }
                    }
                } else if q > 0 {
                    s = q;
                    while s > 0 {
                        let b = c[s - 1];
                        if b.is_ascii_digit() || b == '.' || b == '\u{3c0}' || b == 'e' {
                            s -= 1;
                        } else {
                            break;
                        }
                    }
                    if s > 0 && c[s - 1] == '-' {
                        let unary = s == 1
                            || c[s - 2] == '('
                            || c[s - 2].is_whitespace()
                            || matches!(
                                c[s - 2],
                                '+' | '\u{2212}' | '\u{d7}' | '\u{f7}' | '^' | ',' | '-'
                            );
                        if unary {
                            s -= 1;
                        }
                    }
                } else {
                    s = q;
                }
            }
        }
        let e = self.forward_operand_end(ci);
        (s, e)
    }

    fn last_bin_op(&self) -> Option<(usize, usize)> {
        let c: Vec<char> = self.entry.chars().collect();
        last_bin_op_chars(&c)
    }

    fn pending_op(&self) -> Option<(usize, usize)> {
        let (s, e) = self.last_bin_op()?;
        let rest = self.slice(e, self.chars_len());
        if rest.trim().is_empty() {
            Some((s, e))
        } else {
            None
        }
    }

    fn digit(&mut self, c: char) {
        if self.error {
            self.full_clear();
        }
        if self.evaluated {
            self.evaluated = false;
            self.entry.clear();
            self.cursor = 0;
        }
        let (s, e) = self.operand_span(self.cursor);
        let span = self.slice(s, e);
        let (sign, bare) = if let Some(r) = span.strip_prefix('-') {
            ("-", r)
        } else {
            ("", span.as_str())
        };
        if bare == "0" {
            if c == '0' {
                return;
            }
            self.replace_span(s, e, &format!("{}{}", sign, c));
            return;
        }
        if span.chars().filter(|x| x.is_ascii_digit()).count() >= 16 {
            return;
        }
        let at = self.cursor;
        self.insert_at(at, &c.to_string());
        self.cursor = at + 1;
    }

    fn dot(&mut self) {
        if self.error {
            self.full_clear();
        }
        if self.evaluated {
            self.evaluated = false;
            self.entry.clear();
            self.cursor = 0;
        }
        let (s, e) = self.operand_span(self.cursor);
        if e > s {
            let span = self.slice(s, e);
            if span.contains('.') {
                return;
            }
            if !span.chars().all(|x| x.is_ascii_digit() || x == '-') {
                return;
            }
            if self.cursor == s {
                if span == "0" || span == "-0" {
                    let sign = if span.starts_with('-') { "-" } else { "" };
                    self.replace_span(s, e, &format!("{}0.", sign));
                }
                return;
            }
            let at = self.cursor;
            self.insert_at(at, ".");
            self.cursor = at + 1;
            return;
        }
        let at = self.cursor;
        self.insert_at(at, "0.");
        self.cursor = at + 2;
    }

    fn insert_char(&mut self, ch: char) {
        if self.error {
            self.full_clear();
        }
        if self.evaluated {
            self.evaluated = false;
            self.entry.clear();
            self.cursor = 0;
        }
        let at = self.cursor;
        self.insert_at(at, &ch.to_string());
        self.cursor = at + 1;
    }

    fn backspace(&mut self) {
        if self.error || self.evaluated || self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let b0 = self.byte_at(self.cursor);
        let b1 = self.byte_at(self.cursor + 1);
        let removed = self.entry[b0..b1].chars().next().unwrap_or(' ');
        self.entry.replace_range(b0..b1, "");
        if removed == '(' {
            if self.slice(self.cursor, self.cursor + 1) == ")" {
                let b0 = self.byte_at(self.cursor);
                let b1 = self.byte_at(self.cursor + 1);
                self.entry.replace_range(b0..b1, "");
            }
            if self.cursor == self.chars_len() {
                const NAMES: &[&str] = &[
                    "sin⁻¹",
                    "cos⁻¹",
                    "tan⁻¹",
                    "csc⁻¹",
                    "sec⁻¹",
                    "cot⁻¹",
                    "cosec⁻¹",
                    "asin",
                    "acos",
                    "atan",
                    "cosec",
                    "sin",
                    "cos",
                    "tan",
                    "csc",
                    "sec",
                    "cot",
                    "sqrt",
                    "exp",
                    "abs",
                    "floor",
                    "ceil",
                    "round",
                    "cbrt",
                    "log2",
                    "sign",
                    "ncr",
                    "npr",
                    "gcd",
                    "lcm",
                    "min",
                    "max",
                    "ln",
                    "log",
                    "√",
                    "10^",
                    "e^",
                    "1/",
                ];
                for name in NAMES {
                    if self.entry.ends_with(name) {
                        let at = self.chars_len() - name.chars().count();
                        self.entry.truncate(self.byte_at(at));
                        self.cursor = at;
                        break;
                    }
                }
            }
        }
    }

    fn delete_forward(&mut self) {
        if self.error || self.evaluated || self.cursor >= self.chars_len() {
            return;
        }
        let b0 = self.byte_at(self.cursor);
        let b1 = self.byte_at(self.cursor + 1);
        self.entry.replace_range(b0..b1, "");
    }

    fn press_op(&mut self, op: Op) {
        if self.error {
            return;
        }
        if let Some((os, oe)) = self.pending_op() {
            let old = self.cursor;
            let at_end = old >= self.chars_len();
            self.replace_span(os, oe, op.symbol());
            self.cursor = if at_end {
                self.chars_len()
            } else {
                old.min(self.chars_len())
            };
            self.evaluated = false;
            return;
        }
        if self.entry.is_empty() {
            return;
        }
        let sym = op.symbol();
        self.cursor = self.chars_len();
        let at = self.cursor;
        self.insert_at(at, &format!(" {} ", sym));
        self.cursor = at + sym.chars().count() + 2;
        self.evaluated = false;
    }

    fn equals(&mut self) -> Option<f64> {
        if self.error {
            return None;
        }
        let src: String;
        let path: u8;
        if let Some((os, oe)) = self.pending_op() {
            let c: Vec<char> = self.entry.chars().collect();
            let mut j = os;
            while j > 0 && c[j - 1].is_whitespace() {
                j -= 1;
            }
            let (s2, _) = self.operand_span(j);
            let operand = self.slice(s2, j);
            let lhs = self.slice(0, os);
            let op_text = self.slice(os, oe);
            src = format!("{} {} {}", lhs.trim_end(), op_text, operand);
            path = 1;
        } else if self.evaluated {
            let (sym, rhs) = self.last_op.clone()?;
            src = format!("{} {} {}", self.entry.trim_end(), sym, rhs);
            path = 2;
        } else if self.entry.trim().is_empty() {
            return None;
        } else {
            src = self.entry.trim_end().to_string();
            path = 0;
        }
        match eval_str(&src, self.angle_deg, &[], true) {
            Ok(v) if v.is_finite() => {
                let base = balance_parens(&src);
                self.last_expr = format!("{} =", base);
                let c: Vec<char> = src.chars().collect();
                self.last_op = last_bin_op_chars(&c).map(|(os, oe)| {
                    let sym: String = c[os..oe].iter().collect();
                    let rhs: String = c[oe..].iter().collect();
                    (sym, rhs.trim().to_string())
                });
                self.entry = format_f64(v).replace(',', "");
                self.cursor = self.chars_len();
                self.evaluated = true;
                self.result = v;
                self.has_result = true;
                if path == 2 {
                    return None;
                }
                if is_plain_number(&src) { None } else { Some(v) }
            }
            Ok(_) => {
                self.error = true;
                self.evaluated = false;
                None
            }
            Err(_) => {
                self.error = true;
                self.evaluated = false;
                None
            }
        }
    }

    fn percent(&mut self) {
        if self.error {
            return;
        }
        let (s, e) = self.operand_span(self.cursor);
        if s == e {
            return;
        }
        let t_txt = self.slice(s, e);
        let Ok(t_val) = eval_str(&t_txt, self.angle_deg, &[], true) else {
            return;
        };
        if !t_val.is_finite() {
            return;
        }
        let c: Vec<char> = self.entry.chars().collect();
        let gov = last_bin_op_chars(&c).filter(|(_, oe)| *oe <= s);
        let pct = match gov {
            Some((os, _)) if matches!(c[os], '+' | '\u{2212}') => {
                let lhs: String = c[..os].iter().collect();
                match eval_str(lhs.trim(), self.angle_deg, &[], true) {
                    Ok(lv) if lv.is_finite() => lv * t_val / 100.0,
                    _ => t_val / 100.0,
                }
            }
            _ => t_val / 100.0,
        };
        if !pct.is_finite() {
            return;
        }
        let text = format_f64(pct).replace(',', "");
        self.replace_span(s, e, &text);
        self.evaluated = false;
    }

    fn negate(&mut self) {
        if self.error || self.entry.is_empty() {
            return;
        }
        let (s, e) = self.operand_span(self.cursor);
        if s == e {
            let at = self.cursor;
            self.insert_at(at, "-");
            self.cursor = at + 1;
            self.evaluated = false;
            return;
        }
        let t = self.slice(s, e);
        if let Some(rest) = t.strip_prefix('-') {
            self.replace_span(s, e, rest);
        } else {
            self.replace_span(s, e, &format!("-{}", t));
        }
        self.evaluated = false;
    }

    fn constant(&mut self, v: f64) {
        if self.error {
            return;
        }
        let txt = if v == std::f64::consts::PI {
            "\u{3c0}".to_string()
        } else if v == std::f64::consts::E {
            "e".to_string()
        } else if v == GOLDEN {
            "\u{3c6}".to_string()
        } else {
            format_f64(v).replace(',', "")
        };
        if self.evaluated {
            self.evaluated = false;
            self.entry.clear();
            self.cursor = 0;
        }
        let at = self.cursor;
        self.insert_at(at, &txt);
        self.cursor = at + txt.chars().count();
    }

    fn unary(&mut self, kind: FnKind) {
        if self.error {
            return;
        }
        let second = self.second;
        self.second = false;
        match kind {
            FnKind::Square | FnKind::Cube | FnKind::Fact => {
                if second {
                    return;
                }
                let ch = match kind {
                    FnKind::Square => '\u{b2}',
                    FnKind::Cube => '\u{b3}',
                    _ => '!',
                };
                let (s, e) = self.operand_span(self.cursor);
                if s == e {
                    return;
                }
                let c: Vec<char> = self.entry.chars().collect();
                let n = c.len();
                let mut end = e;
                loop {
                    if end < n && (c[end] == '\u{b2}' || c[end] == '\u{b3}' || c[end] == '!') {
                        end += 1;
                        continue;
                    }
                    if end < n && c[end] == ')' && Self::match_open(&c, end).is_some_and(|o| o < s)
                    {
                        end += 1;
                        continue;
                    }
                    break;
                }
                self.insert_at(end, &ch.to_string());
                self.cursor = end + 1;
                self.evaluated = false;
            }
            _ => self.begin_function(kind, second),
        }
    }

    fn begin_function(&mut self, kind: FnKind, second: bool) {
        let name: &str = match kind {
            FnKind::Sin if second => "sin⁻¹(",
            FnKind::Sin => "sin(",
            FnKind::Cos if second => "cos⁻¹(",
            FnKind::Cos => "cos(",
            FnKind::Tan if second => "tan⁻¹(",
            FnKind::Tan => "tan(",
            FnKind::Csc if second => "csc⁻¹(",
            FnKind::Csc => "csc(",
            FnKind::Sec if second => "sec⁻¹(",
            FnKind::Sec => "sec(",
            FnKind::Cot if second => "cot⁻¹(",
            FnKind::Cot => "cot(",
            FnKind::Ln => "ln(",
            FnKind::Log => "log(",
            FnKind::Sqrt => "√(",
            FnKind::Inv => "1/(",
            FnKind::Exp => "e^(",
            FnKind::Exp10 => "10^(",
            _ => return,
        };
        self.insert_fn_text(name);
    }

    fn insert_fn_text(&mut self, name: &str) {
        if self.evaluated {
            self.evaluated = false;
        }
        let (s, e) = self.operand_span(self.cursor);
        if e > s {
            self.insert_at(e, ")");
            self.insert_at(s, name);
            self.cursor = s + name.chars().count() + (e - s);
        } else {
            let at = self.cursor;
            self.insert_at(at, name);
            self.cursor = at + name.chars().count();
            self.insert_at(self.cursor, ")");
        }
    }

    fn begin_named(&mut self, base: &str) {
        if self.error {
            return;
        }
        let name = format!("{}(", base);
        self.insert_fn_text(&name);
    }

    fn insert_ans(&mut self) {
        if self.error || !self.has_result || self.evaluated {
            return;
        }
        let txt = format_f64(self.result).replace(',', "");
        let at = self.cursor;
        self.insert_at(at, &txt);
        self.cursor = at + txt.chars().count();
    }

    fn open_paren(&mut self) {
        if self.error {
            return;
        }
        if self.evaluated {
            self.evaluated = false;
            self.entry.clear();
            self.cursor = 0;
        }
        let at = self.cursor;
        self.insert_at(at, "(");
        self.cursor = at + 1;
    }

    fn close_paren(&mut self) {
        if self.error {
            return;
        }
        if !has_open_func(&self.entry) {
            return;
        }
        let opens = self.entry.matches('(').count();
        let closes = self.entry.matches(')').count();
        let missing = opens - closes;
        if missing == 0 {
            return;
        }
        let at = self.cursor;
        self.insert_at(at, &")".repeat(missing));
        self.cursor = at + missing;
        self.evaluated = false;
    }

    fn clear_label(&self) -> &'static str {
        if self.error {
            return "C";
        }
        let (s, e) = self.operand_span(self.cursor);
        let t = self.slice(s, e);
        if t.is_empty() {
            if self.entry.trim().is_empty() {
                "AC"
            } else {
                "C"
            }
        } else if is_plain_zero(&t) {
            "AC"
        } else {
            "C"
        }
    }

    fn clear_press(&mut self) {
        if self.clear_label() == "AC" {
            self.full_clear();
            return;
        }
        if self.error {
            self.error = false;
            self.entry.clear();
            self.cursor = 0;
            self.evaluated = false;
            self.last_expr.clear();
            return;
        }
        let (s, e) = self.operand_span(self.cursor);
        if s == e {
            let at = self.cursor;
            self.insert_at(at, "0");
            self.cursor = at + 1;
        } else {
            self.replace_span(s, e, "0");
        }
        self.evaluated = false;
        self.last_expr.clear();
    }

    fn full_clear(&mut self) {
        self.entry.clear();
        self.cursor = 0;
        self.evaluated = false;
        self.error = false;
        self.last_expr.clear();
        self.last_op = None;
        self.second = false;
    }

    fn expression_string(&self) -> String {
        if self.error || !self.evaluated {
            String::new()
        } else {
            self.last_expr.clone()
        }
    }

    #[cfg(test)]
    fn display_string(&self) -> String {
        self.display_and_map().0
    }

    fn display_and_map(&self) -> (String, Vec<usize>) {
        if self.error {
            return ("Error".to_string(), Vec::new());
        }
        if self.entry.is_empty() {
            return ("0".to_string(), vec![0, 0]);
        }
        let (text, mut map) = display_with_map(self.entry.trim_end());
        if let Some(l) = map.last_mut() {
            *l = self.entry.chars().count();
        }
        (text, map)
    }

    fn copy_text(&self) -> String {
        if self.error {
            return "Error".to_string();
        }
        self.entry.trim_end().to_string()
    }

    fn paste(&mut self, text: &str) {
        let cleaned: String = text.trim().replace(',', "");
        if let Ok(v) = cleaned.parse::<f64>() {
            if !v.is_finite() {
                return;
            }
            self.full_clear();
            self.entry = format_f64(v).replace(',', "");
            self.cursor = self.chars_len();
        }
    }

    fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn cursor_right(&mut self) {
        if self.cursor < self.chars_len() {
            self.cursor += 1;
        }
    }

    fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn cursor_end(&mut self) {
        self.cursor = self.chars_len();
    }

    fn set_cursor(&mut self, ci: usize) {
        self.cursor = ci.min(self.chars_len());
    }

    fn next_slot(&mut self) {
        let mut slots: Vec<usize> = vec![0, self.chars_len()];
        for (i, ch) in self.entry.chars().enumerate() {
            if ch == '(' {
                slots.push(i + 1);
            }
            if ch == ')' {
                slots.push(i);
            }
        }
        slots.sort_unstable();
        slots.dedup();
        let next = slots
            .iter()
            .copied()
            .find(|&x| x > self.cursor)
            .unwrap_or(slots[0]);
        self.cursor = next;
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
    Char(char),
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
    F(&'static str),
    Const(f64),
    Ans,
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
    let mut expr = balance_parens(&expr);
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
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "STIXTwoMath".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/STIXTwoMath.otf"
            ))),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("STIXTwoMath".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);
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
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(w, h)));
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
                | Act::F(_)
                | Act::Const(_)
                | Act::Ans
                | Act::Rand
                | Act::Second
                | Act::Char(_)
                | Act::Angle => {}
            }
            return;
        }
        match act {
            Act::Digit(c) => self.engine.digit(c),
            Act::Char(c) => self.engine.insert_char(c),
            Act::Dot => self.engine.dot(),
            Act::Op(op) => self.engine.press_op(op),
            Act::Percent => self.engine.percent(),
            Act::Eq => {
                if let Some(v) = self.engine.equals() {
                    let expr = self.engine.expression_string();
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
            Act::F(name) => self.engine.begin_named(name),
            Act::Const(v) => self.engine.constant(v),
            Act::Ans => self.engine.insert_ans(),
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
        if self.mode == Mode::Programmer {
            return match c {
                '0'..='9' => Some(Act::Digit(c)),
                '+' => Some(Act::Op(Op::Add)),
                '-' | '\u{2212}' => Some(Act::Op(Op::Sub)),
                '*' | '\u{d7}' => Some(Act::Op(Op::Mul)),
                '/' | '\u{f7}' => Some(Act::Op(Op::Div)),
                '%' => Some(Act::Percent),
                '=' | '\r' | '\n' => Some(Act::Eq),
                'a'..='f' => Some(Act::Hex(c.to_ascii_uppercase())),
                'A'..='F' => Some(Act::Hex(c)),
                _ => None,
            };
        }
        match c {
            '0'..='9' => Some(Act::Digit(c)),
            '.' => Some(Act::Dot),
            '+' => Some(Act::Op(Op::Add)),
            '-' | '\u{2212}' => Some(Act::Op(Op::Sub)),
            '*' | '\u{d7}' => Some(Act::Op(Op::Mul)),
            '/' | '\u{f7}' => Some(Act::Op(Op::Div)),
            '^' => Some(Act::Op(Op::Pow)),
            '%' => Some(Act::Percent),
            '=' | '\r' | '\n' => Some(Act::Eq),
            '(' if self.mode == Mode::Scientific => Some(Act::Open),
            ')' if self.mode == Mode::Scientific => Some(Act::Close),
            _ if !c.is_control() => Some(Act::Char(c.to_ascii_lowercase())),
            _ => None,
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        enum Nav {
            Left,
            Right,
            Home,
            End,
            NextSlot,
            Delete,
        }
        let mut chars: Vec<char> = Vec::new();
        let mut keys: Vec<Act> = Vec::new();
        let mut navs: Vec<Nav> = Vec::new();
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
                        egui::Key::Tab => navs.push(Nav::NextSlot),
                        egui::Key::ArrowLeft => navs.push(Nav::Left),
                        egui::Key::ArrowRight => navs.push(Nav::Right),
                        egui::Key::Home => navs.push(Nav::Home),
                        egui::Key::End => navs.push(Nav::End),
                        egui::Key::Delete => navs.push(Nav::Delete),
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
        if !matches!(self.mode, Mode::Programmer) {
            for nav in navs {
                match nav {
                    Nav::Left => self.engine.cursor_left(),
                    Nav::Right => self.engine.cursor_right(),
                    Nav::Home => self.engine.cursor_home(),
                    Nav::End => self.engine.cursor_end(),
                    Nav::NextSlot => self.engine.next_slot(),
                    Nav::Delete => self.engine.delete_forward(),
                }
            }
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

    fn paint_entry(&mut self, ui: &mut egui::Ui) {
        let (text, map) = self.engine.display_and_map();
        let caret = if map.is_empty() {
            None
        } else {
            Some(caret_display_index(&map, self.engine.cursor))
        };
        let avail = ui.available_width();
        let mut size = 52.0;
        let galley = loop {
            let font = egui::FontId::proportional(size);
            let g = ui
                .painter()
                .layout_no_wrap(text.clone(), font, Color32::WHITE);
            if g.size().x <= avail || size <= 16.0 {
                break g;
            }
            size -= 4.0;
        };
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(avail, galley.size().y.max(40.0)),
            egui::Sense::click(),
        );
        let origin = egui::pos2(
            rect.right() - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(origin, galley.clone(), Color32::WHITE);
        if let Some(d) = caret {
            let r = galley.pos_from_cursor(egui::text::CCursor::new(d));
            let x = origin.x + r.min.x;
            let y0 = origin.y + r.min.y;
            let y1 = origin.y + r.max.y;
            ui.painter().line_segment(
                [egui::pos2(x, y0), egui::pos2(x, y1)],
                egui::Stroke::new(2.0, Color32::WHITE),
            );
        }
        if !map.is_empty()
            && let Some(p) = resp.interact_pointer_pos()
        {
            let local = egui::vec2(p.x - origin.x, p.y - origin.y);
            let d = usize::from(galley.cursor_from_pos(local).index);
            self.engine.set_cursor(click_cursor(&map, d));
        }
    }

    fn draw_display(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        match self.mode {
            Mode::Basic | Mode::Scientific => {
                let expr = self.engine.expression_string();
                self.paint_display(ui, &expr, 17.0, DIM, 22.0);
                self.paint_entry(ui);
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

    fn fn_col1_rows(&self) -> Vec<Vec<Cell>> {
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
                trig(FnKind::Csc, "csc", "csc⁻¹"),
                trig(FnKind::Sec, "sec", "sec⁻¹"),
                trig(FnKind::Cot, "cot", "cot⁻¹"),
                (1.0, "1/x", Kind::Fn, 17.0, Act::Fn(FnKind::Inv)),
            ],
            vec![
                (1.0, "ln", Kind::Fn, 18.0, Act::Fn(FnKind::Ln)),
                (1.0, "log", Kind::Fn, 17.0, Act::Fn(FnKind::Log)),
                (1.0, "log₂", Kind::Fn, 16.0, Act::F("log2")),
                (1.0, "√", Kind::Fn, 20.0, Act::Fn(FnKind::Sqrt)),
            ],
            vec![
                (1.0, "x²", Kind::Fn, 18.0, Act::Fn(FnKind::Square)),
                (1.0, "x³", Kind::Fn, 18.0, Act::Fn(FnKind::Cube)),
                (1.0, "xʸ", Kind::Fn, 18.0, Act::Op(Op::Pow)),
                (1.0, "∛", Kind::Fn, 20.0, Act::F("cbrt")),
            ],
            vec![
                (1.0, "|x|", Kind::Fn, 16.0, Act::F("abs")),
                (1.0, "⌊x⌋", Kind::Fn, 17.0, Act::F("floor")),
                (1.0, "⌈x⌉", Kind::Fn, 17.0, Act::F("ceil")),
                (1.0, "rnd", Kind::Fn, 16.0, Act::F("round")),
            ],
        ]
    }

    fn fn_col2_rows(&self) -> Vec<Vec<Cell>> {
        let blank: Cell = (1.0, "", Kind::Disabled, 17.0, Act::Clear);
        vec![
            vec![
                (1.0, "π", Kind::Fn, 20.0, Act::Const(std::f64::consts::PI)),
                (1.0, "e", Kind::Fn, 20.0, Act::Const(std::f64::consts::E)),
                (1.0, "φ", Kind::Fn, 20.0, Act::Const(GOLDEN)),
                (1.0, "mod", Kind::Fn, 16.0, Act::Op(Op::Mod)),
            ],
            vec![
                (1.0, "gcd", Kind::Fn, 16.0, Act::F("gcd")),
                (1.0, "lcm", Kind::Fn, 16.0, Act::F("lcm")),
                (1.0, "nCr", Kind::Fn, 16.0, Act::F("ncr")),
                (1.0, "nPr", Kind::Fn, 16.0, Act::F("npr")),
            ],
            vec![
                (1.0, "eˣ", Kind::Fn, 18.0, Act::Fn(FnKind::Exp)),
                (1.0, "10ˣ", Kind::Fn, 16.0, Act::Fn(FnKind::Exp10)),
                (1.0, ",", Kind::Fn, 22.0, Act::Char(',')),
                (1.0, "Ans", Kind::Fn, 16.0, Act::Ans),
            ],
            vec![
                (1.0, "min", Kind::Fn, 16.0, Act::F("min")),
                (1.0, "max", Kind::Fn, 16.0, Act::F("max")),
                (1.0, "sgn", Kind::Fn, 16.0, Act::F("sign")),
                (1.0, "Rand", Kind::Fn, 15.0, Act::Rand),
            ],
            vec![blank, blank, blank, blank],
            vec![blank, blank, blank, blank],
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
                let rows = basic.len();
                let avail = ui.available_height();
                let h = ((avail - (rows as f32 - 1.0) * GAP) / rows as f32).max(36.0);
                for row in &basic {
                    let w = ui.available_width();
                    if let Some(act) = draw_row(ui, w, row, h) {
                        clicked = Some(act);
                    }
                }
            }
            Mode::Scientific => {
                let c1 = self.fn_col1_rows();
                let c2 = self.fn_col2_rows();
                let avail = ui.available_height();
                let h_of = |n: usize| ((avail - (n as f32 - 1.0) * GAP) / n as f32).max(36.0);
                let h1 = h_of(c1.len());
                let h2 = h_of(c2.len());
                let hb = h_of(basic.len());
                ui.columns(3, |cols| {
                    let w0 = cols[0].available_width();
                    let w1 = cols[1].available_width();
                    let w2 = cols[2].available_width();
                    if let Some(act) = draw_column(&mut cols[0], w0, &c1, h1) {
                        clicked = Some(act);
                    }
                    if let Some(act) = draw_column(&mut cols[1], w1, &c2, h2) {
                        clicked = Some(act);
                    }
                    if let Some(act) = draw_column(&mut cols[2], w2, &basic, hb) {
                        clicked = Some(act);
                    }
                });
            }
            Mode::Programmer => {
                let fns = self.prog_fn_rows();
                let rows = fns.len().max(basic.len());
                let avail = (ui.available_height() - GAP - 32.0).max(36.0 * rows as f32);
                let h = ((avail - (rows as f32 - 1.0) * GAP) / rows as f32).max(36.0);
                ui.columns(2, |cols| {
                    let w0 = cols[0].available_width();
                    let w1 = cols[1].available_width();
                    if let Some(act) = draw_column(&mut cols[0], w0, &fns, h) {
                        clicked = Some(act);
                    }
                    if let Some(act) = draw_column(&mut cols[1], w1, &basic, h) {
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
                Act::Char(c) => self.insert_char(c),
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
                Act::F(name) => self.begin_named(name),
                Act::Const(v) => self.constant(v),
                Act::Ans => self.insert_ans(),
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
        assert_eq!(run(&acts).display_string(), "5!");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "120");
        let mut acts = digits("9");
        acts.push(Act::Fn(FnKind::Sqrt));
        assert_eq!(run(&acts).display_string(), "√(9)");
        acts.push(Act::Eq);
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
        assert_eq!(e.display_string(), "200 + 0");
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
        assert_eq!(e.expression_string(), "");
        assert_eq!(e.display_string(), "12 + 3");
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
        assert_eq!(e.expression_string(), "");
        assert_eq!(e.display_string(), "12 ×");
    }

    #[test]
    fn expression_inside_paren() {
        let mut acts = vec![Act::Open];
        acts.extend(digits("3"));
        let e = run(&acts);
        assert_eq!(e.expression_string(), "");
        assert_eq!(e.display_string(), "(3");
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

    #[test]
    fn trig_degrees() {
        let mut acts = digits("30");
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).display_string(), "sin(30)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "0.5");
        let mut acts = digits("60");
        acts.push(Act::Fn(FnKind::Cos));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "0.5");
        let mut acts = digits("45");
        acts.push(Act::Fn(FnKind::Tan));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "1");
    }

    #[test]
    fn sin_first_shows_function_input() {
        let mut acts = vec![Act::Fn(FnKind::Sin)];
        assert_eq!(run(&acts).display_string(), "sin()");
        acts.extend(digits("30"));
        assert_eq!(run(&acts).display_string(), "sin(30)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "0.5");
        assert_eq!(run(&acts).expression_string(), "sin(30) =");
    }

    #[test]
    fn sin_first_inside_expression() {
        let mut acts = digits("5");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("30"));
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).expression_string(), "");
        assert_eq!(run(&acts).display_string(), "5 + sin(30)");
        acts.push(Act::Eq);
        let mut e = run(&acts);
        assert_eq!(e.display_string(), "5.5");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "6");
    }

    #[test]
    fn nested_functions_with_close_paren() {
        let mut acts = vec![Act::Fn(FnKind::Sin), Act::Fn(FnKind::Cos)];
        assert_eq!(run(&acts).display_string(), "sin(cos())");
        acts.extend(digits("0"));
        acts.push(Act::Close);
        assert_eq!(run(&acts).display_string(), "sin(cos(0))");
        acts.push(Act::Eq);
        let v: f64 = run(&acts)
            .display_string()
            .replace(',', "")
            .parse()
            .unwrap();
        let expected = (0f64.to_radians()).cos().to_radians().sin();
        assert!((v - expected).abs() < 1e-12);
    }

    #[test]
    fn close_paren_balances_function_text() {
        let mut acts = vec![Act::Fn(FnKind::Sqrt)];
        acts.extend(digits("16"));
        acts.push(Act::Close);
        assert_eq!(run(&acts).display_string(), "√(16)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "4");
    }

    #[test]
    fn backspace_removes_whole_function() {
        let acts = vec![Act::Fn(FnKind::Sin), Act::Backspace];
        assert_eq!(run(&acts).display_string(), "0");
        assert!(run(&acts).entry.is_empty());
        let acts = vec![Act::Fn(FnKind::Cos), Act::Backspace, Act::Backspace];
        assert_eq!(run(&acts).display_string(), "0");
    }

    #[test]
    fn constant_appends_inside_function() {
        let mut acts = vec![Act::Fn(FnKind::Sin)];
        acts.push(Act::Const(std::f64::consts::PI));
        assert_eq!(run(&acts).display_string(), "sin(π)");
        acts.push(Act::Eq);
        let v: f64 = run(&acts)
            .display_string()
            .replace(',', "")
            .parse()
            .unwrap();
        let expected = std::f64::consts::PI.to_radians().sin();
        assert!((v - expected).abs() < 1e-12);
    }

    #[test]
    fn trig_after_operator_in_expression() {
        let mut acts = digits("5");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("60"));
        acts.push(Act::Fn(FnKind::Cos));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "5.5");
    }

    #[test]
    fn trig_right_after_operator_uses_slot_zero() {
        let mut acts = digits("5");
        acts.push(Act::Op(Op::Add));
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).display_string(), "5 + sin()");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "5");
    }

    #[test]
    fn inverse_trig_degrees() {
        let mut acts = digits("0");
        acts.push(Act::Dot);
        acts.extend(digits("5"));
        acts.push(Act::Second);
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).display_string(), "sin⁻¹(0.5)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "30");
    }

    #[test]
    fn trig_radians_mode() {
        let mut acts = vec![Act::Angle];
        acts.extend(digits("1.5707963"));
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).display_string(), "sin(1.5707963)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "1");
    }

    #[test]
    fn function_shows_expression_prompt() {
        let mut acts = digits("30");
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).expression_string(), "");
        assert_eq!(run(&acts).display_string(), "sin(30)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).expression_string(), "sin(30) =");
        let mut acts = digits("0");
        acts.push(Act::Dot);
        acts.extend(digits("5"));
        acts.push(Act::Second);
        acts.push(Act::Fn(FnKind::Sin));
        assert_eq!(run(&acts).expression_string(), "");
        assert_eq!(run(&acts).display_string(), "sin⁻¹(0.5)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).expression_string(), "sin⁻¹(0.5) =");
    }

    #[test]
    fn sqrt_after_operator_in_expression() {
        let mut acts = digits("9");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("16"));
        acts.push(Act::Fn(FnKind::Sqrt));
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "13");
    }

    #[test]
    fn parser_basic_expressions() {
        assert_eq!(eval_str("1+2*3", false, &[], false).unwrap(), 7.0);
        assert_eq!(eval_str("2^3^2", false, &[], false).unwrap(), 512.0);
        assert_eq!(eval_str("-2^2", false, &[], false).unwrap(), -4.0);
        assert_eq!(eval_str("5!", false, &[], false).unwrap(), 120.0);
        assert_eq!(eval_str("(1+2)*3", false, &[], false).unwrap(), 9.0);
        assert_eq!(eval_str("2(3+4)", false, &[], false).unwrap(), 14.0);
    }

    #[test]
    fn parser_trig_and_angle_modes() {
        assert!((eval_str("sin(30)", true, &[], false).unwrap() - 0.5).abs() < 1e-12);
        assert!((eval_str("sin(π/2)", false, &[], false).unwrap() - 1.0).abs() < 1e-12);
        assert!((eval_str("sin⁻¹(0.5)", true, &[], false).unwrap() - 30.0).abs() < 1e-9);
        assert!((eval_str("√(16)+√(9)", false, &[], false).unwrap() - 7.0).abs() < 1e-12);
    }

    #[test]
    fn parser_lenient_partial_entry() {
        assert!((eval_str("sin(30", true, &[], true).unwrap() - 0.5).abs() < 1e-12);
        assert!((eval_str("5+", false, &[], true).unwrap() - 5.0).abs() < 1e-12);
        assert!((eval_str("sin(", true, &[], true).unwrap() - 0.0).abs() < 1e-12);
        assert!(eval_str("", false, &[], true).is_err());
        assert!(eval_str("5+", false, &[], false).is_err());
        assert!(eval_str("2**3", false, &[], false).is_err());
    }

    #[test]
    fn balance_and_open_func() {
        assert_eq!(balance_parens("sin(30"), "sin(30)");
        assert_eq!(balance_parens("sin(30)"), "sin(30)");
        assert_eq!(balance_parens("(1+2"), "(1+2)");
        assert!(has_open_func("sin(30"));
        assert!(!has_open_func("sin(30)"));
        assert!(!has_open_func("30"));
        assert_eq!(display_with_map("-").0, "-");
        assert_eq!(display_with_map("1234.5").0, "1,234.5");
        assert_eq!(display_with_map("sin(30").0, "sin(30");
    }

    #[test]
    fn display_with_map_builds_cursor_map() {
        assert_eq!(
            display_with_map("1234"),
            ("1,234".to_string(), vec![0, 1, 1, 2, 3, 4])
        );
        assert_eq!(
            display_with_map("1234.5"),
            ("1,234.5".to_string(), vec![0, 1, 1, 2, 3, 4, 5, 6])
        );
        assert_eq!(
            display_with_map("12 +"),
            ("12 +".to_string(), vec![0, 1, 2, 3, 4])
        );
        assert_eq!(
            display_with_map("sin(30"),
            ("sin(30".to_string(), vec![0, 1, 2, 3, 4, 5, 6])
        );
        assert_eq!(display_with_map(""), (String::new(), vec![0]));
    }

    #[test]
    fn caret_display_index_tracks_entry_cursor() {
        let (_, map) = display_with_map("1234");
        assert_eq!(caret_display_index(&map, 0), 0);
        assert_eq!(caret_display_index(&map, 1), 2);
        assert_eq!(caret_display_index(&map, 2), 3);
        assert_eq!(caret_display_index(&map, 3), 4);
        assert_eq!(caret_display_index(&map, 4), 5);
    }

    #[test]
    fn click_round_trips_entry_cursor() {
        let (_, map) = display_with_map("1234");
        for ci in 0..=4 {
            let d = caret_display_index(&map, ci);
            assert_eq!(click_cursor(&map, d), ci);
        }
        assert_eq!(click_cursor(&map, 6), 4);
    }

    #[test]
    fn empty_and_error_display_maps() {
        let e = Engine::new();
        let (text, map) = e.display_and_map();
        assert_eq!(text, "0");
        assert_eq!(map, vec![0, 0]);
        assert_eq!(caret_display_index(&map, e.cursor), 1);
        assert_eq!(click_cursor(&map, 1), 0);
        let mut err = Engine::new();
        err.error = true;
        let (text, map) = err.display_and_map();
        assert_eq!(text, "Error");
        assert!(map.is_empty());
        assert_eq!(caret_display_index(&map, 0), 0);
        assert_eq!(click_cursor(&map, 3), 0);
    }

    #[test]
    fn trailing_whitespace_maps_end_to_entry_end() {
        let mut e = Engine::new();
        e.entry = "12 + ".to_string();
        e.cursor = 5;
        let (text, map) = e.display_and_map();
        assert_eq!(text, "12 +");
        assert_eq!(map, vec![0, 1, 2, 3, 5]);
        assert_eq!(caret_display_index(&map, e.cursor), 4);
        assert_eq!(click_cursor(&map, 4), 5);
    }

    #[test]
    fn multi_arg_functions() {
        assert_eq!(eval_str("ncr(5, 2)", false, &[], false).unwrap(), 10.0);
        assert_eq!(eval_str("npr(5, 2)", false, &[], false).unwrap(), 20.0);
        assert_eq!(eval_str("gcd(12, 18)", false, &[], false).unwrap(), 6.0);
        assert_eq!(eval_str("gcd(0, 7)", false, &[], false).unwrap(), 7.0);
        assert_eq!(eval_str("lcm(4, 6)", false, &[], false).unwrap(), 12.0);
        assert_eq!(eval_str("min(3, 9)", false, &[], false).unwrap(), 3.0);
        assert_eq!(eval_str("max(3, 9, 4)", false, &[], false).unwrap(), 9.0);
        assert_eq!(eval_str("min(1, 2, 3)", false, &[], false).unwrap(), 1.0);
    }

    #[test]
    fn new_single_arg_functions() {
        assert_eq!(eval_str("floor(-2.7)", false, &[], false).unwrap(), -3.0);
        assert_eq!(eval_str("ceil(-2.7)", false, &[], false).unwrap(), -2.0);
        assert_eq!(eval_str("round(2.5)", false, &[], false).unwrap(), 3.0);
        assert_eq!(eval_str("cbrt(-8)", false, &[], false).unwrap(), -2.0);
        assert_eq!(eval_str("log2(8)", false, &[], false).unwrap(), 3.0);
        assert_eq!(eval_str("sign(-5)", false, &[], false).unwrap(), -1.0);
        assert_eq!(eval_str("sign(0)", false, &[], false).unwrap(), 0.0);
        assert_eq!(eval_str("phi", false, &[], false).unwrap(), GOLDEN);
        assert_eq!(eval_str("φ", false, &[], false).unwrap(), GOLDEN);
    }

    #[test]
    fn argument_arity_errors() {
        assert!(eval_str("ncr(5)", false, &[], false).is_err());
        assert!(eval_str("gcd(5)", false, &[], false).is_err());
        assert!(eval_str("min(5)", false, &[], false).is_err());
        assert!(eval_str("sin(1, 2)", false, &[], false).is_err());
        assert!(eval_str("ncr(2, 5)", false, &[], false).is_err());
    }

    #[test]
    fn named_button_inserts_function() {
        let mut e = run(&[Act::F("cbrt")]);
        assert_eq!(e.entry, "cbrt()");
        assert_eq!(e.cursor, 5);
        for a in digits("27") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "cbrt(27)");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "3");
    }

    #[test]
    fn named_button_wraps_current_operand() {
        let mut e = run(&digits("9"));
        e.run_act(Act::F("floor"));
        assert_eq!(e.entry, "floor(9)");
        assert_eq!(e.cursor, 7);
        e.run_act(Act::Op(Op::Sub));
        e.run_act(Act::F("cbrt"));
        assert_eq!(e.entry, "floor(9) − cbrt()");
        for a in digits("64") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "floor(9) − cbrt(64)");
        e.run_act(Act::Eq);
        assert!((e.result - 5.0).abs() < 1e-12);
    }

    #[test]
    fn ans_inserts_last_result() {
        let mut acts = digits("6");
        acts.push(Act::Op(Op::Mul));
        acts.extend(digits("7"));
        acts.push(Act::Eq);
        let mut e = run(&acts);
        assert_eq!(e.display_string(), "42");
        e.run_act(Act::Op(Op::Add));
        e.run_act(Act::Ans);
        assert_eq!(e.entry, "42 + 42");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "84");
    }

    #[test]
    fn ans_requires_prior_result() {
        let mut e = Engine::new();
        e.run_act(Act::Ans);
        assert_eq!(e.entry, "");
        let mut acts = digits("1");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("1"));
        acts.push(Act::Eq);
        let mut e = run(&acts);
        e.run_act(Act::Ans);
        assert_eq!(e.entry, "2");
    }

    #[test]
    fn golden_constant_inserts_phi() {
        let mut e = Engine::new();
        e.run_act(Act::Const(GOLDEN));
        assert_eq!(e.entry, "φ");
        e.run_act(Act::Op(Op::Mul));
        for a in digits("2") {
            e.run_act(a);
        }
        e.run_act(Act::Eq);
        assert!((e.result - 2.0 * GOLDEN).abs() < 1e-12);
    }

    #[test]
    fn backspace_strips_new_function_names() {
        let mut e = run(&[Act::F("floor")]);
        assert_eq!(e.entry, "floor()");
        e.run_act(Act::Backspace);
        e.run_act(Act::Backspace);
        e.run_act(Act::Backspace);
        e.run_act(Act::Backspace);
        e.run_act(Act::Backspace);
        assert_eq!(e.entry, "");
        assert_eq!(e.cursor, 0);
    }

    #[test]
    fn tab_slots_cycle_through_entry() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        acts.extend(digits("34"));
        let mut e = run(&acts);
        assert_eq!(e.cursor, e.chars_len());
        e.next_slot();
        assert_eq!(e.cursor, 0);
        e.next_slot();
        assert_eq!(e.cursor, e.chars_len());
        let mut e = run(&[Act::Fn(FnKind::Sin)]);
        for a in digits("30") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "sin(30)");
        assert_eq!(e.cursor, 6);
        e.next_slot();
        assert_eq!(e.cursor, 7);
        e.next_slot();
        assert_eq!(e.cursor, 0);
        e.next_slot();
        assert_eq!(e.cursor, 4);
        e.next_slot();
        assert_eq!(e.cursor, 6);
    }

    #[test]
    fn cursor_moves_inside_entry() {
        let mut e = run(&digits("123"));
        e.cursor_home();
        assert_eq!(e.cursor, 0);
        e.cursor_right();
        assert_eq!(e.cursor, 1);
        e.cursor_end();
        assert_eq!(e.cursor, 3);
        e.cursor_left();
        assert_eq!(e.cursor, 2);
        e.set_cursor(1);
        for a in digits("9") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "1923");
        assert_eq!(e.cursor, 2);
    }

    #[test]
    fn pending_operator_swaps_in_place() {
        let mut acts = digits("12");
        acts.push(Act::Op(Op::Add));
        let mut e = run(&acts);
        assert_eq!(e.entry, "12 + ");
        e.set_cursor(4);
        e.run_act(Act::Op(Op::Sub));
        assert_eq!(e.entry, "12 − ");
        assert_eq!(e.cursor, 4);
        e.cursor_end();
        for a in digits("4") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "12 − 4");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "8");
    }

    #[test]
    fn cursor_mid_expression_operates_on_tail() {
        let mut e = run(&digits("50"));
        e.run_act(Act::Op(Op::Add));
        for a in digits("30") {
            e.run_act(a);
        }
        e.set_cursor(2);
        for a in digits("7") {
            e.run_act(a);
        }
        assert_eq!(e.entry, "507 + 30");
        e.cursor_end();
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "537");
    }

    #[test]
    fn typed_text_appends_at_cursor() {
        let mut e = Engine::new();
        e.run_act(Act::Char('s'));
        e.run_act(Act::Char('i'));
        e.run_act(Act::Char('n'));
        assert_eq!(e.entry, "sin");
        assert_eq!(e.cursor, 3);
        e.run_act(Act::Open);
        for a in digits("30") {
            e.run_act(a);
        }
        e.run_act(Act::Close);
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "0.5");
    }

    #[test]
    fn delete_forward_removes_char_at_cursor() {
        let mut e = run(&digits("123"));
        e.set_cursor(1);
        e.delete_forward();
        assert_eq!(e.entry, "13");
        assert_eq!(e.cursor, 1);
    }

    #[test]
    fn equals_duplicates_operand_when_op_pending() {
        let mut acts = digits("200");
        acts.push(Act::Op(Op::Add));
        let mut e = run(&acts);
        assert_eq!(e.equals(), Some(400.0));
        assert_eq!(e.display_string(), "400");
        e.run_act(Act::Eq);
        assert_eq!(e.display_string(), "600");
    }

    #[test]
    fn csc_sec_cot_round_trip() {
        assert!(
            (eval_str("csc(30)", true, &[], false).unwrap() - 2.0).abs() < 1e-9,
            "csc(30deg) = 2"
        );
        assert!(
            (eval_str("sec(60)", true, &[], false).unwrap() - 2.0).abs() < 1e-9,
            "sec(60deg) = 2"
        );
        assert!(
            (eval_str("cot(45)", true, &[], false).unwrap() - 1.0).abs() < 1e-9,
            "cot(45deg) = 1"
        );
        assert!(
            (eval_str("csc⁻¹(2)", true, &[], false).unwrap() - 30.0).abs() < 1e-9,
            "csc⁻¹(2) = 30"
        );
        assert!(
            (eval_str("sec⁻¹(2)", true, &[], false).unwrap() - 60.0).abs() < 1e-9,
            "sec⁻¹(2) = 60"
        );
        assert!(
            (eval_str("cot⁻¹(1)", true, &[], false).unwrap() - 45.0).abs() < 1e-9,
            "cot⁻¹(1) = 45"
        );
    }

    #[test]
    fn csc_button_and_square_cube_fact() {
        let mut acts = digits("30");
        acts.push(Act::Fn(FnKind::Csc));
        assert_eq!(run(&acts).display_string(), "csc(30)");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "2");
        let mut acts = digits("3");
        acts.push(Act::Fn(FnKind::Cube));
        assert_eq!(run(&acts).display_string(), "3³");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "27");
        let mut acts = digits("4");
        acts.push(Act::Fn(FnKind::Square));
        assert_eq!(run(&acts).display_string(), "4²");
        acts.push(Act::Eq);
        assert_eq!(run(&acts).display_string(), "16");
    }

    fn tv(expr: &str, deg: bool, want: f64, tol: f64) {
        let got = eval_str(expr, deg, &[], false).unwrap_or_else(|e| panic!("{expr}: {e}"));
        assert!((got - want).abs() <= tol, "{expr}: got {got}, want {want}");
    }

    fn tv2(lhs: &str, rhs: &str, deg: bool, tol: f64) {
        let a = eval_str(lhs, deg, &[], false).unwrap_or_else(|e| panic!("{lhs}: {e}"));
        let b = eval_str(rhs, deg, &[], false).unwrap_or_else(|e| panic!("{rhs}: {e}"));
        assert!((a - b).abs() <= tol, "{lhs} = {a}, {rhs} = {b}");
    }

    #[test]
    fn trig_special_angles_deg() {
        let s = [
            ("sin(0)", 0.0),
            ("sin(30)", 0.5),
            ("sin(45)", std::f64::consts::FRAC_1_SQRT_2),
            ("sin(60)", 0.8660254037844386),
            ("sin(90)", 1.0),
            ("sin(120)", 0.8660254037844386),
            ("sin(135)", std::f64::consts::FRAC_1_SQRT_2),
            ("sin(150)", 0.5),
            ("sin(180)", 0.0),
            ("sin(210)", -0.5),
            ("sin(225)", -std::f64::consts::FRAC_1_SQRT_2),
            ("sin(240)", -0.8660254037844386),
            ("sin(270)", -1.0),
            ("sin(300)", -0.8660254037844386),
            ("sin(315)", -std::f64::consts::FRAC_1_SQRT_2),
            ("sin(330)", -0.5),
            ("sin(360)", 0.0),
        ];
        for (e, w) in s {
            tv(e, true, w, 1e-12);
        }
        let c = [
            ("cos(0)", 1.0),
            ("cos(30)", 0.8660254037844386),
            ("cos(45)", std::f64::consts::FRAC_1_SQRT_2),
            ("cos(60)", 0.5),
            ("cos(90)", 0.0),
            ("cos(120)", -0.5),
            ("cos(135)", -std::f64::consts::FRAC_1_SQRT_2),
            ("cos(150)", -0.8660254037844386),
            ("cos(180)", -1.0),
            ("cos(210)", -0.8660254037844386),
            ("cos(225)", -std::f64::consts::FRAC_1_SQRT_2),
            ("cos(240)", -0.5),
            ("cos(270)", 0.0),
            ("cos(300)", 0.5),
            ("cos(315)", std::f64::consts::FRAC_1_SQRT_2),
            ("cos(330)", 0.8660254037844386),
            ("cos(360)", 1.0),
        ];
        for (e, w) in c {
            tv(e, true, w, 1e-12);
        }
        let t = [
            ("tan(0)", 0.0),
            ("tan(30)", 0.5773502691896257),
            ("tan(45)", 1.0),
            ("tan(60)", 1.7320508075688772),
            ("tan(120)", -1.7320508075688772),
            ("tan(135)", -1.0),
            ("tan(150)", -0.5773502691896257),
            ("tan(180)", 0.0),
            ("tan(210)", 0.5773502691896257),
            ("tan(225)", 1.0),
            ("tan(240)", 1.7320508075688772),
            ("tan(300)", -1.7320508075688772),
            ("tan(315)", -1.0),
            ("tan(330)", -0.5773502691896257),
            ("tan(360)", 0.0),
        ];
        for (e, w) in t {
            tv(e, true, w, 1e-12);
        }
    }

    #[test]
    fn trig_special_values_rad() {
        let cases = [
            ("sin(π/6)", 0.5),
            ("sin(π/4)", std::f64::consts::FRAC_1_SQRT_2),
            ("sin(π/3)", 0.8660254037844386),
            ("sin(π/2)", 1.0),
            ("sin(π)", 0.0),
            ("sin(3*π/2)", -1.0),
            ("cos(0)", 1.0),
            ("cos(π/6)", 0.8660254037844386),
            ("cos(π/4)", std::f64::consts::FRAC_1_SQRT_2),
            ("cos(π/3)", 0.5),
            ("cos(π)", -1.0),
            ("cos(2*π)", 1.0),
            ("tan(0)", 0.0),
            ("tan(π/4)", 1.0),
            ("tan(π/3)", 1.7320508075688772),
            ("tan(π)", 0.0),
        ];
        for (e, w) in cases {
            tv(e, false, w, 1e-12);
        }
    }

    #[test]
    fn trig_pythagorean_identities() {
        for x in [0.0, 17.0, 30.0, 45.0, 60.0, 90.0, 137.0, 300.0] {
            let e = format!("sin({x})^2+cos({x})^2");
            tv(&e, true, 1.0, 1e-12);
        }
        for x in [17.0, 30.0, 45.0, 60.0, 77.0, 137.0, 200.0, 300.0] {
            let l = format!("1+tan({x})^2");
            let r = format!("1/cos({x})^2");
            tv2(&l, &r, true, 1e-9);
            let l = format!("1+cot({x})^2");
            let r = format!("csc({x})^2");
            tv2(&l, &r, true, 1e-9);
        }
    }

    #[test]
    fn trig_double_angle_identities() {
        for x in [10.0, 20.0, 37.0, 45.0, 60.0, 75.0, 120.0, 135.0] {
            let l = format!("sin(2*{x})");
            let r = format!("2*sin({x})*cos({x})");
            tv2(&l, &r, true, 1e-11);
            let l = format!("cos(2*{x})");
            let r = format!("cos({x})^2-sin({x})^2");
            tv2(&l, &r, true, 1e-11);
            let r = format!("2*cos({x})^2-1");
            tv2(&l, &r, true, 1e-11);
        }
        for x in [10.0, 20.0, 37.0, 60.0, 75.0] {
            let l = format!("tan(2*{x})");
            let r = format!("2*tan({x})/(1-tan({x})^2)");
            tv2(&l, &r, true, 1e-9);
        }
    }

    #[test]
    fn trig_cofunction_identities() {
        for x in [5.0, 17.0, 30.0, 45.0, 63.0, 80.0] {
            let l = format!("sin({x})");
            let r = format!("cos(90-{x})");
            tv2(&l, &r, true, 1e-12);
            let l = format!("tan({x})");
            let r = format!("cot(90-{x})");
            tv2(&l, &r, true, 1e-9);
            let l = format!("sec({x})");
            let r = format!("csc(90-{x})");
            tv2(&l, &r, true, 1e-9);
        }
    }

    #[test]
    fn trig_parity_identities() {
        for x in [10.0, 33.0, 45.0, 67.0, 90.0] {
            let l = format!("sin(-{x})");
            let r = format!("-sin({x})");
            tv2(&l, &r, true, 1e-12);
            let l = format!("cos(-{x})");
            let r = format!("cos({x})");
            tv2(&l, &r, true, 1e-12);
            let l = format!("tan(-{x})");
            let r = format!("-tan({x})");
            tv2(&l, &r, true, 1e-11);
        }
    }

    #[test]
    fn trig_reciprocal_identities() {
        for x in [10.0, 30.0, 45.0, 60.0, 77.0, 120.0] {
            let l = format!("csc({x})");
            let r = format!("1/sin({x})");
            tv2(&l, &r, true, 1e-11);
            let l = format!("sec({x})");
            let r = format!("1/cos({x})");
            tv2(&l, &r, true, 1e-11);
            let l = format!("cot({x})");
            let r = format!("1/tan({x})");
            tv2(&l, &r, true, 1e-9);
        }
    }

    #[test]
    fn trig_inverse_functions_deg() {
        tv("asin(0.5)", true, 30.0, 1e-9);
        tv("asin(1)", true, 90.0, 1e-9);
        tv("asin(0)", true, 0.0, 1e-9);
        tv("asin(-0.5)", true, -30.0, 1e-9);
        tv("acos(0.5)", true, 60.0, 1e-9);
        tv("acos(1)", true, 0.0, 1e-9);
        tv("acos(0)", true, 90.0, 1e-9);
        tv("acos(-0.5)", true, 120.0, 1e-9);
        tv("atan(0)", true, 0.0, 1e-9);
        tv("atan(1)", true, 45.0, 1e-9);
        tv("atan(-1)", true, -45.0, 1e-9);
        tv("atan(√3)", true, 60.0, 1e-9);
        tv("atan(1/√3)", true, 30.0, 1e-9);
        tv("sin⁻¹(0.5)", true, 30.0, 1e-9);
        tv("cos⁻¹(0.5)", true, 60.0, 1e-9);
        tv("tan⁻¹(1)", true, 45.0, 1e-9);
        tv("sin⁻¹(sin(47))", true, 47.0, 1e-9);
        tv("cos⁻¹(cos(23))", true, 23.0, 1e-9);
        tv("tan⁻¹(tan(65))", true, 65.0, 1e-9);
    }

    #[test]
    fn trig_inverse_functions_rad() {
        tv("asin(1)", false, std::f64::consts::FRAC_PI_2, 1e-12);
        tv("acos(0)", false, std::f64::consts::FRAC_PI_2, 1e-12);
        tv("atan(1)", false, std::f64::consts::FRAC_PI_4, 1e-12);
        tv("sin⁻¹(0.5)", false, std::f64::consts::PI / 6.0, 1e-12);
        tv("cos⁻¹(0.5)", false, std::f64::consts::PI / 3.0, 1e-12);
    }

    #[test]
    fn trig_sum_difference_identities() {
        for x in [20.0, 40.0, 65.0] {
            let l = format!("sin({x}+15)");
            let r = format!("sin({x})*cos(15)+cos({x})*sin(15)");
            tv2(&l, &r, true, 1e-11);
            let l = format!("cos({x}+15)");
            let r = format!("cos({x})*cos(15)-sin({x})*sin(15)");
            tv2(&l, &r, true, 1e-11);
            let l = format!("sin({x}-15)");
            let r = format!("sin({x})*cos(15)-cos({x})*sin(15)");
            tv2(&l, &r, true, 1e-11);
            let l = format!("cos({x}-15)");
            let r = format!("cos({x})*cos(15)+sin({x})*sin(15)");
            tv2(&l, &r, true, 1e-11);
        }
        for x in [20.0, 40.0] {
            let l = format!("tan({x}+15)");
            let r = format!("(tan({x})+tan(15))/(1-tan({x})*tan(15))");
            tv2(&l, &r, true, 1e-9);
        }
    }

    #[test]
    fn trig_half_angle_identities() {
        for x in [30.0, 80.0, 200.0] {
            let l = format!("sin({x}/2)");
            let r = format!("√((1-cos({x}))/2)");
            tv2(&l, &r, true, 1e-12);
        }
        for x in [30.0, 80.0] {
            let l = format!("cos({x}/2)");
            let r = format!("√((1+cos({x}))/2)");
            tv2(&l, &r, true, 1e-12);
        }
    }
}
