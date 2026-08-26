//! Reading one colour notation and reporting it in all of them.

use alloc::format;
use alloc::string::String;

use crate::jscompat::float::to_js_string;

/// A colour, held as the reference holds it: channels that may be fractional.
///
/// `rgb(1.5, 2, 3)` is accepted and reported back with its fraction intact,
/// while the hex form rounds. Storing integers here would lose that before the
/// difference could be shown.
struct Colour {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

/// Parses whichever notation the input carries and renders the rest.
///
/// Nothing recognised is not a failure: the channels stay at their opening
/// values and the operation reports black at full alpha. The reference has no
/// else-branch, so an unparseable input and a literal black are the same
/// answer.
pub(super) fn parse(input: &str) -> String {
    let colour = read(input);
    render(&colour)
}

/// Tries each notation in the reference's order and takes the first that hits.
fn read(input: &str) -> Colour {
    if let Some(colour) = read_hex(input) {
        return colour;
    }
    if let Some(colour) = read_rgb(input) {
        return colour;
    }
    if let Some(colour) = read_hsl(input) {
        return colour;
    }
    if let Some(colour) = read_cmyk(input) {
        return colour;
    }
    Colour {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    }
}

/// `#rrggbb` anywhere in the input, case-insensitive.
///
/// Unanchored, like the reference's pattern: a hex colour inside a longer
/// string is found rather than rejected, and only the first six digits after
/// a `#` are read even when more follow.
fn read_hex(input: &str) -> Option<Colour> {
    let bytes = input.as_bytes();
    for start in 0..bytes.len() {
        if bytes[start] != b'#' {
            continue;
        }
        let digits = &bytes.get(start + 1..start + 7)?;
        if digits.len() < 6 || !digits.iter().all(u8::is_ascii_hexdigit) {
            continue;
        }
        return Some(Colour {
            red: f64::from(hex_pair(digits[0], digits[1])),
            green: f64::from(hex_pair(digits[2], digits[3])),
            blue: f64::from(hex_pair(digits[4], digits[5])),
            alpha: 1.0,
        });
    }
    None
}

/// Two hex digits as a byte.
fn hex_pair(high: u8, low: u8) -> u8 {
    let value = |digit: u8| char::from(digit).to_digit(16).unwrap_or(0);
    u8::try_from(value(high) * 16 + value(low)).unwrap_or(0)
}

/// `rgb(r, g, b)` or `rgba(r, g, b, a)`.
fn read_rgb(input: &str) -> Option<Colour> {
    let numbers = read_call(input, "rgba")
        .or_else(|| read_call(input, "rgb"))?;
    Some(Colour {
        red: *numbers.first()?,
        green: *numbers.get(1)?,
        blue: *numbers.get(2)?,
        alpha: numbers.get(3).copied().unwrap_or(1.0),
    })
}

/// `hsl(h, s%, l%)` or `hsla(h, s%, l%, a)`.
fn read_hsl(input: &str) -> Option<Colour> {
    let numbers = read_call(input, "hsla")
        .or_else(|| read_call(input, "hsl"))?;
    let hue = numbers.first()? / 360.0;
    let saturation = numbers.get(1)? / 100.0;
    let lightness = numbers.get(2)? / 100.0;
    let (red, green, blue) = hsl_to_rgb(hue, saturation, lightness);
    Some(Colour {
        red,
        green,
        blue,
        alpha: numbers.get(3).copied().unwrap_or(1.0),
    })
}

/// `cmyk(c, m, y, k)`, whose channels run from zero to one.
fn read_cmyk(input: &str) -> Option<Colour> {
    let numbers = read_call(input, "cmyk")?;
    let cyan = *numbers.first()?;
    let magenta = *numbers.get(1)?;
    let yellow = *numbers.get(2)?;
    let black = *numbers.get(3)?;
    Some(Colour {
        red: (255.0 * (1.0 - cyan) * (1.0 - black)).round(),
        green: (255.0 * (1.0 - magenta) * (1.0 - black)).round(),
        blue: (255.0 * (1.0 - yellow) * (1.0 - black)).round(),
        alpha: 1.0,
    })
}

/// Finds `name(...)` and reads the comma-separated numbers inside it.
///
/// Percent signs are stepped over rather than required, because the only
/// notation that carries them puts them in fixed places and the reference's
/// pattern would have failed on the whole call if one were missing.
fn read_call(input: &str, name: &str) -> Option<alloc::vec::Vec<f64>> {
    let lowered = input.to_ascii_lowercase();
    let start = lowered.find(&format!("{name}("))?;
    let open = start + name.len() + 1;
    let close = lowered[open..].find(')')? + open;
    let mut numbers = alloc::vec::Vec::new();
    for piece in lowered[open..close].split(',') {
        let trimmed = piece.trim().trim_end_matches('%');
        numbers.push(trimmed.parse::<f64>().ok()?);
    }
    Some(numbers)
}

/// The reference's HSL-to-RGB, which rounds each channel on the way out.
fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
    if saturation == 0.0 {
        let grey = (lightness * 255.0).round();
        return (grey, grey, grey);
    }
    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;
    (
        (hue_to_channel(p, q, hue + 1.0 / 3.0) * 255.0).round(),
        (hue_to_channel(p, q, hue) * 255.0).round(),
        (hue_to_channel(p, q, hue - 1.0 / 3.0) * 255.0).round(),
    )
}

/// One channel of the HSL conversion, wrapping the hue into `0..1` first.
fn hue_to_channel(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// The reference's RGB-to-HSL, in the `0..1` range it returns.
///
/// The equality tests are exact on purpose. The reference decides the
/// achromatic case with `===` and picks the hue branch with a `switch` over
/// the maximum, both of which are exact comparisons; a tolerance here would
/// take a different branch than the reference on the values that sit at the
/// boundary, which is where a colour conversion is most likely to be checked.
#[expect(
    clippy::float_cmp,
    clippy::manual_midpoint,
    reason = "the reference's branches turn on exact equality and on (max + min) / 2"
)]
fn rgb_to_hsl(red: f64, green: f64, blue: f64) -> (f64, f64, f64) {
    let (red, green, blue) = (red / 255.0, green / 255.0, blue / 255.0);
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let lightness = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, lightness);
    }
    let delta = max - min;
    let saturation = if lightness > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };
    // The branch is on which channel *is* the maximum, so a tie goes to the
    // first of them -- red before green before blue, as the reference's
    // `switch` does.
    let hue = if max == red {
        (green - blue) / delta + if green < blue { 6.0 } else { 0.0 }
    } else if max == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    (hue / 6.0, saturation, lightness)
}

/// Renders every notation, plus the picker the reference embeds.
fn render(colour: &Colour) -> String {
    let (hue, saturation, lightness) = rgb_to_hsl(colour.red, colour.green, colour.blue);
    let hue = (hue * 360.0).round();
    let saturation = (saturation * 100.0).round();
    let lightness = (lightness * 100.0).round();

    let black = 1.0 - (colour.red / 255.0).max(colour.green / 255.0).max(colour.blue / 255.0);
    // A fully black colour leaves `1 - k` at zero, so each of these is a
    // division by zero. The reference tests the result rather than the divisor
    // and prints a bare `0`, which is not the two-decimal form the others get.
    let cyan = ink(1.0 - colour.red / 255.0 - black, black);
    let magenta = ink(1.0 - colour.green / 255.0 - black, black);
    let yellow = ink(1.0 - colour.blue / 255.0 - black, black);

    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        channel_byte(colour.red),
        channel_byte(colour.green),
        channel_byte(colour.blue)
    );
    let red = to_js_string(colour.red);
    let green = to_js_string(colour.green);
    let blue = to_js_string(colour.blue);
    let alpha = to_js_string(colour.alpha);
    let rgb = format!("rgb({red}, {green}, {blue})");
    let rgba = format!("rgba({red}, {green}, {blue}, {alpha})");
    let hue = to_js_string(hue);
    let saturation = to_js_string(saturation);
    let lightness = to_js_string(lightness);
    let hsl = format!("hsl({hue}, {saturation}%, {lightness}%)");
    let hsla = format!("hsla({hue}, {saturation}%, {lightness}%, {alpha})");
    let cmyk = format!("cmyk({cyan}, {magenta}, {yellow}, {})", fixed2(black));

    format!(
        "<div id=\"colorpicker\" style=\"white-space: normal;\"></div>\n\
         Hex:  {hex}\n\
         RGB:  {rgb}\n\
         RGBA: {rgba}\n\
         HSL:  {hsl}\n\
         HSLA: {hsla}\n\
         CMYK: {cmyk}\n\
         <script>\n    \
         $('#colorpicker').colorpicker({{\n        \
         format: 'rgba',\n        \
         color: '{rgba}',\n        \
         container: true,\n        \
         inline: true,\n        \
         useAlpha: true\n    \
         }}).on('colorpickerChange', function(e) {{\n        \
         var color = e.color.string('rgba');\n        \
         window.app.manager.input.setInput(color);\n        \
         window.app.manager.input.inputChange(new Event(\"keyup\"));\n    \
         }});\n\
         </script>"
    )
}

/// One CMYK ink, or a bare `0` where the division had no answer.
fn ink(numerator: f64, black: f64) -> String {
    let value = numerator / (1.0 - black);
    if value.is_nan() {
        String::from("0")
    } else {
        fixed2(value)
    }
}

/// Two decimal places, the way `Number.prototype.toFixed(2)` writes them.
fn fixed2(value: f64) -> String {
    format!("{value:.2}")
}

/// A channel rounded into the byte the hex form shows.
///
/// Clamped before the cast rather than after, so the conversion is only ever
/// asked about a value it can hold. A channel outside `0..=255` is reachable:
/// nothing validates `rgb(-5, 300, 0)`, and the reference lets such a value
/// through to `toString(16)`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 0..=255 on the lines above"
)]
fn channel_byte(value: f64) -> u8 {
    let rounded = value.round();
    if rounded <= 0.0 {
        0
    } else if rounded >= 255.0 {
        255
    } else {
        rounded as u8
    }
}
