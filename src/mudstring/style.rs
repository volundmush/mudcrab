use crate::mudstring::{
    color::{Color, ColorType, ColorSystem}
};

use std::collections::{HashMap};
use xmltree::Element;

use html_escape::{
    encode_text
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Link {
    pub id: i64,
    pub url: String
}

impl Link {
    pub fn new(url: impl AsRef<str>) -> Self {
        Self {
            id: 0,
            url: url.as_ref().to_string()
        }
    }
}


#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub color: Option<Color>,
    pub bgcolor: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub blink2: bool,
    pub reverse: bool,
    pub conceal: bool,
    pub strike: bool,
    pub underline2: bool,
    pub frame: bool,
    pub encircle: bool,
    pub overline: bool,
    pub link: Option<Link>,
    pub element: Option<Element>
}

impl Style {

    pub fn set_mxp(&mut self, elem: Element) {
        self.element = Some(elem);

    }

    pub fn clear_mxp(&mut self) {
        self.element = None;
    }

    pub fn set_link(&mut self, url: impl AsRef<str>) {
        self.link = Some(Link::new(url));
    }

    pub fn clear_link(&mut self) {
        self.link = None;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = Some(color);
    }

    pub fn clear_color(&mut self) {
        self.color = None;
    }

    pub fn set_bgcolor(&mut self, color: Color) {
        self.bgcolor = Some(color);
    }

    pub fn clear_bgcolor(&mut self) {
        self.bgcolor = None;
    }

    pub fn ansi_codes(&self, system: ColorSystem) -> String {
        let mut out = Vec::new();
        let sys_val = system as u8;

        if let Some(c) = &self.color {
            let col_sys = c.system() as u8;
            if col_sys > sys_val {
                let col = c.downgrade(system);
                out.push(col.get_ansi_codes(true));
            } else {
                out.push(c.get_ansi_codes(true));
            }
        }

        if let Some(c) = &self.bgcolor {
            let col_sys = c.system() as u8;
            if col_sys > sys_val {
                let col = c.downgrade(system);
                out.push(col.get_ansi_codes(false));
            } else {
                out.push(c.get_ansi_codes(false));
            }
        }

        out.join(";")

    }

    pub fn render(&self, text: impl AsRef<str>, system: Option<ColorSystem>, legacy_windows: bool, links: bool, mxp: bool) -> String {
        let t = if mxp {
            encode_text(&text).to_string()
        } else {
            text.as_ref().to_string()
        };
        if t.len() == 0 {
            return t
        }

        let mut rendered = if let Some(sys) = system {
            let attrs = self.ansi_codes(sys);
            format!("\x1b[{}m{}\x1b[0m", attrs, t)
        } else {
            t
        };

        if links & !legacy_windows {
            if let Some(link) = &self.link {
                rendered = format!("\x1b]8;id={};{}\x1b\\{}\x1b]8;;\x1b\\", link.id, link.url, rendered);
            }
        }

        if mxp {
            if let Some(elem) = &self.element {
                if elem.attributes.len() > 0 {

                } else {
                    rendered = format!("\x1b[4z<{}>{}\x1b]4z</{}>", elem.name, rendered, elem.name);
                }
            }
        }

        rendered
    }

}