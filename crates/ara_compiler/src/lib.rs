use std::{env, fs, path::PathBuf};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Ident, LitStr, Token, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

#[derive(Clone, Debug, Default)]
struct CssStyleProps {
    display: Option<String>,
    flex_direction: Option<String>,
    justify_content: Option<String>,
    align_items: Option<String>,
    width: Option<f32>,
    width_pct: Option<f32>,
    height: Option<f32>,
    height_pct: Option<f32>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    padding: Option<[f32; 4]>, // top, right, bottom, left
    margin: Option<[f32; 4]>,
    gap: Option<[f32; 2]>, // row_gap, column_gap
    flex_grow: Option<f32>,
    background_color: Option<[f32; 4]>,
    color: Option<[f32; 4]>,
    font_size: Option<f32>,
}

impl CssStyleProps {
    fn merge_from(&mut self, other: &CssStyleProps) {
        if other.display.is_some() {
            self.display = other.display.clone();
        }
        if other.flex_direction.is_some() {
            self.flex_direction = other.flex_direction.clone();
        }
        if other.justify_content.is_some() {
            self.justify_content = other.justify_content.clone();
        }
        if other.align_items.is_some() {
            self.align_items = other.align_items.clone();
        }
        if other.width.is_some() {
            self.width = other.width;
            self.width_pct = None;
        }
        if other.width_pct.is_some() {
            self.width_pct = other.width_pct;
            self.width = None;
        }
        if other.height.is_some() {
            self.height = other.height;
            self.height_pct = None;
        }
        if other.height_pct.is_some() {
            self.height_pct = other.height_pct;
            self.height = None;
        }
        if other.min_width.is_some() {
            self.min_width = other.min_width;
        }
        if other.min_height.is_some() {
            self.min_height = other.min_height;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.margin.is_some() {
            self.margin = other.margin;
        }
        if other.gap.is_some() {
            self.gap = other.gap;
        }
        if other.flex_grow.is_some() {
            self.flex_grow = other.flex_grow;
        }
        if other.background_color.is_some() {
            self.background_color = other.background_color;
        }
        if other.color.is_some() {
            self.color = other.color;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
    }
}

fn parse_px(val: &str) -> Option<f32> {
    let v = val.trim().trim_end_matches("px").trim();
    v.parse::<f32>().ok()
}

fn parse_box_shorthand(val: &str) -> Option<[f32; 4]> {
    let nums: Vec<f32> = val
        .split_whitespace()
        .filter_map(parse_px)
        .collect();
    match nums.as_slice() {
        [all] => Some([*all, *all, *all, *all]),
        [v, h] => Some([*v, *h, *v, *h]),
        [t, h, b] => Some([*t, *h, *b, *h]),
        [t, r, b, l, ..] => Some([*t, *r, *b, *l]),
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<[f32; 4]> {
    let h = hex.trim().trim_start_matches('#');
    match h.len() {
        3 => {
            let r = u8::from_str_radix(&h[0..1], 16).ok()?;
            let g = u8::from_str_radix(&h[1..2], 16).ok()?;
            let b = u8::from_str_radix(&h[2..3], 16).ok()?;
            Some([
                (r * 17) as f32 / 255.0,
                (g * 17) as f32 / 255.0,
                (b * 17) as f32 / 255.0,
                1.0,
            ])
        }
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        8 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            let a = u8::from_str_radix(&h[6..8], 16).ok()?;
            Some([
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ])
        }
        _ => None,
    }
}

fn parse_color(val: &str) -> Option<[f32; 4]> {
    let v = val.trim();
    if v.starts_with('#') {
        return parse_hex_color(v);
    }
    if let Some(inner) = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<f32> = inner
            .split(',')
            .filter_map(|p| p.trim().parse::<f32>().ok())
            .collect();
        if parts.len() >= 3 {
            let r = if parts[0] > 1.0 { parts[0] / 255.0 } else { parts[0] };
            let g = if parts[1] > 1.0 { parts[1] / 255.0 } else { parts[1] };
            let b = if parts[2] > 1.0 { parts[2] / 255.0 } else { parts[2] };
            let a = parts.get(3).copied().unwrap_or(1.0);
            return Some([r, g, b, a]);
        }
    }
    match v {
        "white" => Some([1.0, 1.0, 1.0, 1.0]),
        "black" => Some([0.0, 0.0, 0.0, 1.0]),
        "transparent" => Some([0.0, 0.0, 0.0, 0.0]),
        _ => None,
    }
}

#[derive(Debug)]
struct CssRule {
    selector: String,
    props: CssStyleProps,
}

fn parse_css_rules(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    let mut rest = css;
    while let Some(open_idx) = rest.find('{') {
        let selector_part = rest[..open_idx].trim();
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = after_open.find('}') else {
            break;
        };
        let body = &after_open[..close_idx];
        rest = &after_open[close_idx + 1..];

        let mut props = CssStyleProps::default();
        for decl in body.split(';') {
            let Some((k, v)) = decl.split_once(':') else {
                continue;
            };
            let key = k.trim();
            let val = v.trim();
            match key {
                "display" => props.display = Some(val.to_string()),
                "flex-direction" => props.flex_direction = Some(val.to_string()),
                "justify-content" => props.justify_content = Some(val.to_string()),
                "align-items" => props.align_items = Some(val.to_string()),
                "width" => {
                    if let Some(pct) = val.strip_suffix('%') {
                        props.width_pct = pct.trim().parse::<f32>().ok().map(|p| p / 100.0);
                    } else {
                        props.width = parse_px(val);
                    }
                }
                "height" => {
                    if let Some(pct) = val.strip_suffix('%') {
                        props.height_pct = pct.trim().parse::<f32>().ok().map(|p| p / 100.0);
                    } else {
                        props.height = parse_px(val);
                    }
                }
                "min-width" => props.min_width = parse_px(val),
                "min-height" => props.min_height = parse_px(val),
                "padding" => props.padding = parse_box_shorthand(val),
                "margin" => props.margin = parse_box_shorthand(val),
                "gap" => {
                    let nums: Vec<f32> = val.split_whitespace().filter_map(parse_px).collect();
                    match nums.as_slice() {
                        [g] => props.gap = Some([*g, *g]),
                        [r, c, ..] => props.gap = Some([*r, *c]),
                        _ => {}
                    }
                }
                "row-gap" => {
                    if let Some(g) = parse_px(val) {
                        let col = props.gap.map(|p| p[1]).unwrap_or(0.0);
                        props.gap = Some([g, col]);
                    }
                }
                "column-gap" => {
                    if let Some(g) = parse_px(val) {
                        let row = props.gap.map(|p| p[0]).unwrap_or(0.0);
                        props.gap = Some([row, g]);
                    }
                }
                "flex-grow" => props.flex_grow = val.parse::<f32>().ok(),
                "background-color" | "background" => props.background_color = parse_color(val),
                "color" => props.color = parse_color(val),
                "font-size" => props.font_size = parse_px(val),
                _ => {}
            }
        }

        for sel in selector_part.split(',') {
            let clean = sel.trim();
            if !clean.is_empty() {
                rules.push(CssRule {
                    selector: clean.to_string(),
                    props: props.clone(),
                });
            }
        }
    }
    rules
}

fn selector_matches(selector: &str, tag: &str, classes: &[String]) -> bool {
    let last_segment = selector.split_whitespace().last().unwrap_or(selector);
    if last_segment == tag {
        return true;
    }
    if last_segment.starts_with('.') {
        let req_classes: Vec<&str> = last_segment
            .split('.')
            .filter(|s| !s.is_empty())
            .collect();
        return !req_classes.is_empty()
            && req_classes
                .iter()
                .all(|rc| classes.iter().any(|c| c == rc));
    }
    if let Some((t, class_part)) = last_segment.split_once('.') {
        if t != tag {
            return false;
        }
        let req_classes: Vec<&str> = class_part.split('.').filter(|s| !s.is_empty()).collect();
        return req_classes
            .iter()
            .all(|rc| classes.iter().any(|c| c == rc));
    }
    false
}

#[derive(Debug, Clone)]
struct HtmlElement {
    tag: String,
    classes: Vec<String>,
    on_click: Option<String>,
    text_content: String,
    children: Vec<HtmlElement>,
}

struct HtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> HtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.rest().chars().next().unwrap();
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else if self.rest().starts_with("<!--") {
                if let Some(end) = self.rest().find("-->") {
                    self.pos += end + 3;
                } else {
                    self.pos = self.input.len();
                }
            } else {
                break;
            }
        }
    }

    fn parse_nodes(&mut self) -> Vec<HtmlElement> {
        let mut nodes = Vec::new();
        loop {
            self.skip_ws();
            if self.pos >= self.input.len() || self.rest().starts_with("</") {
                break;
            }
            if self.rest().starts_with('<') {
                if let Some(el) = self.parse_element() {
                    nodes.push(el);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        nodes
    }

    fn parse_element(&mut self) -> Option<HtmlElement> {
        self.skip_ws();
        if !self.rest().starts_with('<') || self.rest().starts_with("</") {
            return None;
        }
        self.pos += 1; // '<'

        let tag_start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.rest().chars().next().unwrap();
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let tag = self.input[tag_start..self.pos].to_string();

        let mut classes = Vec::new();
        let mut on_click = None;
        let mut self_closing = false;

        loop {
            self.skip_ws();
            if self.pos >= self.input.len() {
                break;
            }
            if self.rest().starts_with("/>") {
                self.pos += 2;
                self_closing = true;
                break;
            }
            if self.rest().starts_with('>') {
                self.pos += 1;
                break;
            }

            let attr_start = self.pos;
            while self.pos < self.input.len() {
                let ch = self.rest().chars().next().unwrap();
                if ch == '=' || ch.is_whitespace() || ch == '>' || ch == '/' {
                    break;
                }
                self.pos += ch.len_utf8();
            }
            let attr_name = self.input[attr_start..self.pos].trim().to_string();
            self.skip_ws();

            let mut attr_val = String::new();
            if self.rest().starts_with('=') {
                self.pos += 1;
                self.skip_ws();
                if self.rest().starts_with('"') || self.rest().starts_with('\'') {
                    let quote_ch = self.rest().chars().next().unwrap();
                    self.pos += 1;
                    let val_start = self.pos;
                    if let Some(end_idx) = self.rest().find(quote_ch) {
                        attr_val = self.rest()[..end_idx].to_string();
                        self.pos = val_start + end_idx + 1;
                    }
                }
            }

            match attr_name.as_str() {
                "class" => {
                    classes.extend(
                        attr_val
                            .split_whitespace()
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                    );
                }
                "(click)" => {
                    on_click = Some(attr_val);
                }
                _ => {}
            }
        }

        if self_closing {
            return Some(HtmlElement {
                tag,
                classes,
                on_click,
                text_content: String::new(),
                children: Vec::new(),
            });
        }

        let mut text_content = String::new();
        let mut children = Vec::new();

        loop {
            if self.pos >= self.input.len() {
                break;
            }
            if self.rest().starts_with("</") {
                if let Some(close_gt) = self.rest().find('>') {
                    self.pos += close_gt + 1;
                } else {
                    self.pos = self.input.len();
                }
                break;
            }
            if self.rest().starts_with("<!--") {
                self.skip_ws();
                continue;
            }
            if self.rest().starts_with('<') {
                if let Some(child) = self.parse_element() {
                    children.push(child);
                } else {
                    break;
                }
            } else {
                let next_lt = self.rest().find('<').unwrap_or(self.rest().len());
                let raw_text = &self.rest()[..next_lt];
                let collapsed = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !collapsed.is_empty() {
                    if !text_content.is_empty() {
                        text_content.push(' ');
                    }
                    text_content.push_str(&collapsed);
                }
                self.pos += next_lt;
            }
        }

        Some(HtmlElement {
            tag,
            classes,
            on_click,
            text_content,
            children,
        })
    }
}

fn translate_interpolation_expr(raw_expr: &str, field_names: &[String]) -> TokenStream2 {
    let trimmed = raw_expr.trim();
    // Angular signal read syntax `srv.clicks()` -> `srv.clicks.get()`
    let normalized = if let Some(without_parens) = trimmed.strip_suffix("()") {
        format!("{without_parens}.get()")
    } else {
        trimmed.to_string()
    };

    let root_ident = normalized
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");

    let expr_tokens: TokenStream2 = normalized
        .parse()
        .expect("Invalid Rust expression inside template interpolation {{ ... }}");

    if field_names.iter().any(|f| f == root_ident) {
        let ident = format_ident!("{}", root_ident);
        quote! {{
            let #ident = self.#ident.clone();
            #expr_tokens
        }}
    } else {
        expr_tokens
    }
}

fn generate_node_mount(
    el: &HtmlElement,
    rules: &[CssRule],
    parent_var: TokenStream2,
    field_names: &[String],
    counter: &mut usize,
) -> TokenStream2 {
    *counter += 1;
    let node_var = format_ident!("node_{}", *counter);

    let mut style = CssStyleProps::default();
    for rule in rules {
        if selector_matches(&rule.selector, &el.tag, &el.classes) {
            style.merge_from(&rule.props);
        }
    }

    let display_ts = match style.display.as_deref() {
        Some("none") => quote! { ara::core::taffy::style::Display::None },
        _ => quote! { ara::core::taffy::style::Display::Flex },
    };

    let flex_dir_ts = match style.flex_direction.as_deref() {
        Some("column") => quote! { ara::core::taffy::style::FlexDirection::Column },
        _ => quote! { ara::core::taffy::style::FlexDirection::Row },
    };

    let justify_ts = match style.justify_content.as_deref() {
        Some("center") => quote! { Some(ara::core::taffy::style::JustifyContent::Center) },
        Some("space-between") => {
            quote! { Some(ara::core::taffy::style::JustifyContent::SpaceBetween) }
        }
        Some("space-around") => {
            quote! { Some(ara::core::taffy::style::JustifyContent::SpaceAround) }
        }
        Some("space-evenly") => {
            quote! { Some(ara::core::taffy::style::JustifyContent::SpaceEvenly) }
        }
        Some("flex-start" | "start") => {
            quote! { Some(ara::core::taffy::style::JustifyContent::FlexStart) }
        }
        Some("flex-end" | "end") => {
            quote! { Some(ara::core::taffy::style::JustifyContent::FlexEnd) }
        }
        _ => quote! { Some(ara::core::taffy::style::JustifyContent::Center) },
    };

    let align_ts = match style.align_items.as_deref() {
        Some("center") => quote! { Some(ara::core::taffy::style::AlignItems::Center) },
        Some("stretch") => quote! { Some(ara::core::taffy::style::AlignItems::Stretch) },
        Some("flex-start" | "start") => {
            quote! { Some(ara::core::taffy::style::AlignItems::FlexStart) }
        }
        Some("flex-end" | "end") => quote! { Some(ara::core::taffy::style::AlignItems::FlexEnd) },
        _ => quote! { Some(ara::core::taffy::style::AlignItems::Center) },
    };

    let width_ts = if let Some(w) = style.width {
        quote! { ara::core::taffy::style::Dimension::Length(#w) }
    } else if let Some(pct) = style.width_pct {
        quote! { ara::core::taffy::style::Dimension::Percent(#pct) }
    } else {
        quote! { ara::core::taffy::style::Dimension::Auto }
    };

    let height_ts = if let Some(h) = style.height {
        quote! { ara::core::taffy::style::Dimension::Length(#h) }
    } else if let Some(pct) = style.height_pct {
        quote! { ara::core::taffy::style::Dimension::Percent(#pct) }
    } else {
        quote! { ara::core::taffy::style::Dimension::Auto }
    };

    let min_width_ts = if let Some(mw) = style.min_width {
        quote! { ara::core::taffy::style::Dimension::Length(#mw) }
    } else {
        quote! { ara::core::taffy::style::Dimension::Auto }
    };

    let min_height_ts = if let Some(mh) = style.min_height {
        quote! { ara::core::taffy::style::Dimension::Length(#mh) }
    } else {
        quote! { ara::core::taffy::style::Dimension::Auto }
    };

    let [pt, pr, pb, pl] = style.padding.unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let [mt, mr, mb, ml] = style.margin.unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let [rgap, cgap] = style.gap.unwrap_or([0.0, 0.0]);
    let flex_grow = style.flex_grow.unwrap_or(0.0);

    let [br, bg, bb, ba] = style.background_color.unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let [tr, tg, tb, ta] = style.color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let text_scale = style
        .font_size
        .map(|px| (px / 6.0).clamp(1.5, 10.0))
        .unwrap_or(3.5);

    let on_click_ts = if let Some(click_expr) = &el.on_click {
        let root_ident = click_expr
            .trim()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        let call_tokens: TokenStream2 = click_expr
            .parse()
            .expect("Invalid Rust expression in (click) handler");

        if field_names.iter().any(|f| f == root_ident) {
            let ident = format_ident!("{}", root_ident);
            quote! {{
                let #ident = self.#ident.clone();
                Some(Box::new(move || {
                    #call_tokens;
                }) as Box<dyn Fn() + Send + Sync>)
            }}
        } else {
            quote! {
                Some(Box::new(move || {
                    #call_tokens;
                }) as Box<dyn Fn() + Send + Sync>)
            }
        }
    } else {
        quote! { None }
    };

    let text_binding_ts = if !el.text_content.is_empty() {
        if el.text_content.contains("{{") && el.text_content.contains("}}") {
            let mut fmt_str = String::new();
            let mut exprs = Vec::new();
            let mut rem = el.text_content.as_str();

            while let Some(open) = rem.find("{{") {
                fmt_str.push_str(&rem[..open]);
                let after_open = &rem[open + 2..];
                if let Some(close) = after_open.find("}}") {
                    fmt_str.push_str("{}");
                    let expr_str = &after_open[..close];
                    exprs.push(translate_interpolation_expr(expr_str, field_names));
                    rem = &after_open[close + 2..];
                } else {
                    fmt_str.push_str(rem);
                    rem = "";
                    break;
                }
            }
            fmt_str.push_str(rem);

            let clone_stmts: Vec<TokenStream2> = field_names
                .iter()
                .map(|f| {
                    let ident = format_ident!("{}", f);
                    quote! { let #ident = self.#ident.clone(); }
                })
                .collect();

            let inner_exprs: Vec<TokenStream2> = el
                .text_content
                .split("{{")
                .skip(1)
                .filter_map(|part| part.split_once("}}").map(|(e, _)| e))
                .map(|raw_expr| {
                    let trimmed = raw_expr.trim();
                    let normalized = if let Some(without_parens) = trimmed.strip_suffix("()") {
                        format!("{without_parens}.get()")
                    } else {
                        trimmed.to_string()
                    };
                    normalized
                        .parse::<TokenStream2>()
                        .expect("Invalid interpolation expression")
                })
                .collect();

            quote! {{
                #(#clone_stmts)*
                let scene_for_effect = ::std::sync::Arc::clone(scene);
                let target_node = #node_var;
                ara::core::reactive_graph::effect::Effect::new(move |_| {
                    let formatted = format!(#fmt_str, #(#inner_exprs),*);
                    scene_for_effect.write().update_text(target_node, formatted);
                });
            }}
        } else {
            let txt = &el.text_content;
            quote! {
                scene.write().update_text(#node_var, #txt);
            }
        }
    } else {
        quote! {}
    };

    let children_ts: Vec<TokenStream2> = el
        .children
        .iter()
        .map(|child| {
            generate_node_mount(
                child,
                rules,
                quote! { Some(#node_var) },
                field_names,
                counter,
            )
        })
        .collect();

    quote! {
        let #node_var = {
            let taffy_style = ara::core::taffy::style::Style {
                display: #display_ts,
                flex_direction: #flex_dir_ts,
                justify_content: #justify_ts,
                align_items: #align_ts,
                size: ara::core::taffy::geometry::Size {
                    width: #width_ts,
                    height: #height_ts,
                },
                min_size: ara::core::taffy::geometry::Size {
                    width: #min_width_ts,
                    height: #min_height_ts,
                },
                padding: ara::core::taffy::geometry::Rect {
                    top: ara::core::taffy::style::LengthPercentage::Length(#pt),
                    right: ara::core::taffy::style::LengthPercentage::Length(#pr),
                    bottom: ara::core::taffy::style::LengthPercentage::Length(#pb),
                    left: ara::core::taffy::style::LengthPercentage::Length(#pl),
                },
                margin: ara::core::taffy::geometry::Rect {
                    top: ara::core::taffy::style::LengthPercentageAuto::Length(#mt),
                    right: ara::core::taffy::style::LengthPercentageAuto::Length(#mr),
                    bottom: ara::core::taffy::style::LengthPercentageAuto::Length(#mb),
                    left: ara::core::taffy::style::LengthPercentageAuto::Length(#ml),
                },
                gap: ara::core::taffy::geometry::Size {
                    width: ara::core::taffy::style::LengthPercentage::Length(#cgap),
                    height: ara::core::taffy::style::LengthPercentage::Length(#rgap),
                },
                flex_grow: #flex_grow,
                ..Default::default()
            };
            let click_cb = #on_click_ts;
            scene.write().add_styled_node(
                #parent_var,
                taffy_style,
                [#br, #bg, #bb, #ba],
                [#tr, #tg, #tb, #ta],
                #text_scale,
                click_cb,
            )
        };
        #text_binding_ts
        #(#children_ts)*
    }
}

struct ComponentFieldSpec {
    name: Ident,
    ty: Type,
}

impl Parse for ComponentFieldSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(Self { name, ty })
    }
}

struct ComponentMacroInput {
    comp_ident: Ident,
    fields: Vec<ComponentFieldSpec>,
    template_path: String,
    style_path: String,
}

impl Parse for ComponentMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut comp_ident = None;
        let mut fields = Vec::new();
        let mut template_path = None;
        let mut style_path = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "component" => {
                    let name: Ident = input.parse()?;
                    if input.peek(syn::token::Brace) {
                        let content;
                        braced!(content in input);
                        let parsed_fields: Punctuated<ComponentFieldSpec, Token![,]> =
                            content.parse_terminated(ComponentFieldSpec::parse, Token![,])?;
                        fields = parsed_fields.into_iter().collect();
                    }
                    comp_ident = Some(name);
                }
                "template" => {
                    let s: LitStr = input.parse()?;
                    template_path = Some(s.value());
                }
                "style" | "styles" => {
                    let s: LitStr = input.parse()?;
                    style_path = Some(s.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown key `{other}` in ara::component!"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            comp_ident: comp_ident.expect("Missing `component` in ara::component!"),
            fields,
            template_path: template_path.expect("Missing `template` in ara::component!"),
            style_path: style_path.expect("Missing `style` in ara::component!"),
        })
    }
}

fn compile_component_impl(
    comp_ident: &Ident,
    fields: &[(Ident, Type)],
    template_rel_path: &str,
    style_rel_path: &str,
) -> TokenStream2 {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let html_full_path = PathBuf::from(&manifest_dir).join(template_rel_path);
    let scss_full_path = PathBuf::from(&manifest_dir).join(style_rel_path);

    let html_src = fs::read_to_string(&html_full_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read component template {}: {}",
            html_full_path.display(),
            e
        )
    });
    let scss_src = fs::read_to_string(&scss_full_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read component style {}: {}",
            scss_full_path.display(),
            e
        )
    });

    let compiled_css = grass::from_string(scss_src, &grass::Options::default())
        .unwrap_or_else(|e| panic!("Failed to compile SCSS {}: {}", scss_full_path.display(), e));

    let rules = parse_css_rules(&compiled_css);
    let mut parser = HtmlParser::new(&html_src);
    let root_elements = parser.parse_nodes();

    let field_names: Vec<String> = fields.iter().map(|(id, _)| id.to_string()).collect();
    let mut counter = 0usize;
    let mount_nodes: Vec<TokenStream2> = root_elements
        .iter()
        .map(|el| {
            generate_node_mount(
                el,
                &rules,
                quote! { parent_id },
                &field_names,
                &mut counter,
            )
        })
        .collect();

    let construct_fields: Vec<TokenStream2> = fields
        .iter()
        .map(|(fname, fty)| {
            quote! {
                #fname: <#fty as ara::core::Construct>::construct(inj)
            }
        })
        .collect();

    let html_path_str = html_full_path.to_string_lossy().to_string();
    let scss_path_str = scss_full_path.to_string_lossy().to_string();

    quote! {
        const _: &str = include_str!(#html_path_str);
        const _: &str = include_str!(#scss_path_str);

        impl ara::core::Construct for #comp_ident {
            fn construct(inj: &ara::core::Injector) -> Self {
                Self {
                    #(#construct_fields),*
                }
            }
        }

        impl ara::core::Mount for #comp_ident {
            fn mount(
                &self,
                scene: &::std::sync::Arc<ara::core::parking_lot::RwLock<ara::core::SceneGraph>>,
                parent_id: ::std::option::Option<ara::core::NodeId>,
            ) {
                use ara::prelude::Get;
                #(#mount_nodes)*
            }
        }

        impl ara::core::DefineComponent for #comp_ident {}
    }
}

#[proc_macro]
pub fn component(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as ComponentMacroInput);
    let fields: Vec<(Ident, Type)> = parsed
        .fields
        .into_iter()
        .map(|f| (f.name, f.ty))
        .collect();
    let expanded = compile_component_impl(
        &parsed.comp_ident,
        &fields,
        &parsed.template_path,
        &parsed.style_path,
    );
    TokenStream::from(expanded)
}

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let comp_ident = &ast.ident;

    let mut template_path = None;
    let mut style_path = None;

    for attr in &ast.attrs {
        if attr.path().is_ident("component") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("template") {
                    let s: LitStr = meta.value()?.parse()?;
                    template_path = Some(s.value());
                } else if meta.path.is_ident("style") || meta.path.is_ident("styles") {
                    let s: LitStr = meta.value()?.parse()?;
                    style_path = Some(s.value());
                }
                Ok(())
            });
        }
    }

    let mut fields = Vec::new();
    if let syn::Data::Struct(data_struct) = &ast.data
        && let syn::Fields::Named(named) = &data_struct.fields
    {
        for field in &named.named {
            if let Some(ident) = &field.ident {
                fields.push((ident.clone(), field.ty.clone()));
            }
        }
    }

    let expanded = compile_component_impl(
        comp_ident,
        &fields,
        &template_path.expect("Missing #[component(template = \"...\")]"),
        &style_path.expect("Missing #[component(style = \"...\")]"),
    );
    TokenStream::from(expanded)
}
