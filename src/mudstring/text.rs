use crate::mudstring::{
    style::{Style},
    color::{ColorSystem}
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub style: Option<Style>
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Segment {
    pub text: String,
    pub style: Option<Style>
}


#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub plain: String,
    pub spans: Vec<Span>
}

impl Default for Text {
    fn default() -> Self {
        let mut spans = Vec::with_capacity(1);
        spans.push(Span {start: 0, end: 0, style: None});
        Self {
            plain: Default::default(),
            spans
        }
    }
}

impl From<Vec<Segment>> for Text {
    fn from(segs: Vec<Segment>) -> Self {
        let mut plain = String::new();
        let mut spans = Vec::with_capacity(segs.len());
        let mut current = plain.len();
        for seg in segs {
            let length = seg.text.len();
            plain.push_str(&seg.text);
            spans.push(Span {start: current, end: current+length, style: seg.style});
            current = plain.len();
        }
        Self {
            plain,
            spans
        }
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        let mut spans = Vec::with_capacity(1);
        let plain = s.to_string();
        let mut span = Span {
            start: 0,
            end: plain.len(),
            style: None
        };
        spans.push(span);
        Self {
            plain,
            spans
        }
    }
}

impl Text {
    pub fn render(&self, system: Option<ColorSystem>, legacy_windows: bool, links: bool, mxp: bool) -> String {
        let mut out = String::new();
        for sp in &self.spans {
            if let Some(c) = &sp.style {
                out.push_str(&c.render(&self.plain[sp.start..sp.end], system, legacy_windows, links, mxp));
            } else {
                out.push_str(&self.plain[sp.start..sp.end]);
            }
        }
        out
    }
}