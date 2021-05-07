use colori::{RgbColor, HslColor};
use serde_derive::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct ColorTriplet {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl ColorTriplet {
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue
        }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:x}{:x}{:x}", self.red, self.green, self.blue)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum ColorSystem {
    Standard = 1,
    EightBit = 2,
    TrueColor = 3,
    Windows = 4
}

impl ColorSystem {
    pub fn to_type(&self) -> ColorType {
        match self {
            ColorSystem::Standard => ColorType::Standard,
            ColorSystem::EightBit => ColorType::EightBit,
            ColorSystem::TrueColor => ColorType::TrueColor,
            ColorSystem::Windows => ColorType::Windows,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum ColorType {
    Default = 0,
    Standard = 1,
    EightBit = 2,
    TrueColor = 3,
    Windows = 4
}

impl ColorType {
    pub fn to_system(&self) -> ColorSystem {
        match self {
            ColorType::Default | ColorType::Standard => ColorSystem::Standard,
            ColorType::EightBit => ColorSystem::EightBit,
            ColorType::TrueColor => ColorSystem::TrueColor,
            ColorType::Windows => ColorSystem::Windows
        }
    }
}

impl Default for ColorType {
    fn default() -> Self {
        Self::Default
    }
}


#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub name: String,
    pub ctype: ColorType,
    pub number: Option<u8>,
    pub triplet: Option<ColorTriplet>
}

impl Default for Color {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            ctype: Default::default(),
            number: None,
            triplet: None
        }
    }
}

impl From<u8> for Color {
    fn from(num: u8) -> Self {
        let ctype = if num < 16 {
            ColorType::Standard
        } else {
            ColorType::EightBit
        };
        Self {
            name: format!("color({})", num),
            ctype,
            number: Some(num),
            triplet: None
        }
    }
}

impl From<ColorTriplet> for Color {
    fn from(t: ColorTriplet) -> Self {
        Self {
            name: t.to_hex(),
            ctype: ColorType::TrueColor,
            number: None,
            triplet: Some(t)
        }
    }
}

impl Color {
    pub fn system(&self) -> ColorSystem {
        self.ctype.to_system()
    }

    pub fn is_system_defined(&self) -> bool {
        match self.system() {
            ColorSystem::EightBit | ColorSystem::TrueColor => {
                false
            }
            _ => {
                true
            }
        }
    }

    pub fn is_default(&self) -> bool {
        self.ctype == ColorType::Default
    }

    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::from(ColorTriplet::new(red, green, blue))
    }

    pub fn get_ansi_codes(&self, foreground: bool) -> String {
        match self.ctype {
            ColorType::Default => {
                if foreground {
                    "39".to_string()
                } else {
                    "49".to_string()
                }
            },
            ColorType::Windows => {
                "0".to_string()
            },
            ColorType::Standard => {
                if let Some(n) = &self.number {
                    if foreground {
                        format!("{}", 30 + n)
                    } else {
                        format!("{}", 40 + n)
                    }
                } else {
                    if foreground {
                        format!("{}", 30)
                    } else {
                        format!("{}", 40)
                    }
                }
            }
            ColorType::EightBit => {
                if foreground {
                    format!("38;5;{}", self.number.unwrap_or_else(|| 0))
                } else {
                    format!("48;5;{}", self.number.unwrap_or_else(|| 0))
                }
            },
            ColorType::TrueColor => {
                if let Some(t) = &self.triplet {
                    if foreground {
                        format!("38;2;{};{};{}", t.red, t.green, t.blue)
                    } else {
                        format!("48;2;{};{};{}", t.red, t.green, t.blue)
                    }
                } else {
                    if foreground {
                        format!("38;2;{};{};{}", 0, 0, 0)
                    } else {
                        format!("48;2;{};{};{}", 0, 0, 0)
                    }
                }
            }
        }
    }

    pub fn downgrade(&self, downto: ColorSystem) -> Self {
        let down_val = downto as u8;
        let sys_val = self.system() as u8;

        if (sys_val == 0) | (down_val == sys_val) {
            return self.clone()
        }

        if (sys_val == 3) & (down_val == 2) {
            // Convert from TRUECOLOR to EIGHTBIT
            let trip = self.triplet.unwrap();
            let mut rgb = RgbColor(trip.red, trip.green, trip.blue);
            let hsl = rgb.into_hsl();

            if hsl.saturation() < 0.1 {
                let mut color_number : u8 = 0;
                let gray = (hsl.lightness() * 25.0).round() as i32;
                if gray == 0 {
                    color_number = 16;
                } else if gray == 25 {
                    color_number = 231;
                } else {
                    color_number = 231 + gray as u8;
                }
                return Color::from(color_number);
            }
            let color_number = (16 + 36 * (trip.red * 5) + 6 * (trip.green * 5) + (trip.blue * 5));
            return Color::from(color_number as u8);
        }

        else if down_val == 1 {
            if sys_val == 3 {

            } else {

            }
        }
        return self.clone()
    }

}