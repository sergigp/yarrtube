/// A color in OKLCH (Björn Ottosson's OKLab, in polar form): lightness from 0
/// to 1, chroma, and hue in degrees from 0 up to 360.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub lightness: f64,
    pub chroma: f64,
    pub hue: f64,
}

impl Oklch {
    /// The OKLCH of a `#rrggbb` color.
    pub fn from_hex(hex: &str) -> Self {
        let [r, g, b] = [1, 3, 5].map(|at| {
            let byte = u8::from_str_radix(hex.get(at..at + 2).unwrap_or("00"), 16).unwrap_or(0);
            gamma_decode(f64::from(byte) / 255.0)
        });
        let l = (0.4122214708 * r + 0.5363329363 * g + 0.0514459929 * b).cbrt();
        let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
        let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
        let lab_a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
        let lab_b = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
        Self {
            lightness: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
            chroma: lab_a.hypot(lab_b),
            hue: lab_b.atan2(lab_a).to_degrees().rem_euclid(360.0),
        }
    }

    /// `#rrggbb` of this color. Where it falls outside sRGB, the chroma drops
    /// until it fits, so the lightness and hue hold.
    pub fn to_hex(self) -> String {
        let in_gamut = std::iter::successors(Some(self.chroma), |chroma| Some(chroma - 0.001))
            .map(|chroma| self.to_linear_rgb(chroma))
            .find(|rgb| rgb.iter().all(|channel| (0.0..=1.0).contains(channel)))
            .unwrap_or_default();
        in_gamut
            .iter()
            .map(|channel| format!("{:02x}", (gamma_encode(*channel) * 255.0).round() as u8))
            .fold("#".to_string(), |hex, byte| hex + &byte)
    }

    /// Linear sRGB channels of this color at `chroma`, unclamped.
    fn to_linear_rgb(self, chroma: f64) -> [f64; 3] {
        let radians = self.hue.to_radians();
        let lab_a = chroma * radians.cos();
        let lab_b = chroma * radians.sin();
        let l = (self.lightness + 0.3963377774 * lab_a + 0.2158037573 * lab_b).powi(3);
        let m = (self.lightness - 0.1055613458 * lab_a - 0.0638541728 * lab_b).powi(3);
        let s = (self.lightness - 0.0894841775 * lab_a - 1.291485548 * lab_b).powi(3);
        [
            4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
        ]
    }
}

fn gamma_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn gamma_decode(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_read_lightness_and_hue_and_make_the_solid_of_that_hue() {
        let pastel_sky = Oklch::from_hex("#d3ecf8");
        let solid_sky = Oklch {
            lightness: 0.62,
            chroma: 0.13,
            ..pastel_sky
        };

        assert_eq!(
            (
                (pastel_sky.lightness * 1000.0).round(),
                pastel_sky.hue.round(),
                solid_sky.to_hex()
            ),
            (928.0, 228.0, "#0193bf".to_string())
        );
    }
}
