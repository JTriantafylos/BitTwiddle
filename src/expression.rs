use std::{fmt, sync::OnceLock};

use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerWidth {
    Eight,
    Sixteen,
    ThirtyTwo,
    SixtyFour,
}

impl Default for IntegerWidth {
    fn default() -> Self {
        Self::SixtyFour
    }
}

impl IntegerWidth {
    pub fn bits(self) -> u32 {
        match self {
            Self::Eight => 8,
            Self::Sixteen => 16,
            Self::ThirtyTwo => 32,
            Self::SixtyFour => 64,
        }
    }

    pub fn mask(self) -> u64 {
        if self.bits() == u64::BITS {
            u64::MAX
        } else {
            (1_u64 << self.bits()) - 1
        }
    }

    pub fn max_value(self) -> u64 {
        self.mask()
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Eight => Self::SixtyFour,
            Self::Sixteen => Self::Eight,
            Self::ThirtyTwo => Self::Sixteen,
            Self::SixtyFour => Self::ThirtyTwo,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Eight => Self::Sixteen,
            Self::Sixteen => Self::ThirtyTwo,
            Self::ThirtyTwo => Self::SixtyFour,
            Self::SixtyFour => Self::Eight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    ShiftLeft,
    ShiftRight,
    BitOr,
    BitAnd,
    BitXor,
}

impl Operator {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Add),
            "-" => Some(Self::Subtract),
            "*" => Some(Self::Multiply),
            "/" => Some(Self::Divide),
            "%" => Some(Self::Modulo),
            "<<" => Some(Self::ShiftLeft),
            ">>" => Some(Self::ShiftRight),
            "|" => Some(Self::BitOr),
            "&" => Some(Self::BitAnd),
            "^" => Some(Self::BitXor),
            _ => None,
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::BitOr => "|",
            Self::BitAnd => "&",
            Self::BitXor => "^",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParsedExpression {
    Literal {
        value: u64,
    },
    Binary {
        operand1: u64,
        operator: Operator,
        operand2: u64,
    },
    UnaryNot {
        operand: u64,
    },
    BitRangeMask {
        operand: u64,
        start_bit: u32,
        end_bit: u32,
    },
}

/// Parses an unsigned integer literal into a `u64`.
///
/// Decimal literals are accepted by default. Prefixes select other bases:
/// `0x`/`0X` for hexadecimal, `0o`/`0O` for octal, and `0b`/`0B` for binary.
/// Returns the underlying integer parse error for empty, invalid, or overflowing
/// literals.
fn parse_unsigned(s: &str) -> Result<u64, std::num::ParseIntError> {
    if let Some(rest) = s.to_lowercase().strip_prefix("0x") {
        u64::from_str_radix(rest, 16)
    } else if let Some(rest) = s.to_lowercase().strip_prefix("0o") {
        u64::from_str_radix(rest, 8)
    } else if let Some(rest) = s.to_lowercase().strip_prefix("0b") {
        u64::from_str_radix(rest, 2)
    } else {
        #[allow(clippy::from_str_radix_10)] // Explicitly state base 10 for completeness sake.
        u64::from_str_radix(s, 10)
    }
}

/// Parses an arithmetic or bitwise expression.
///
/// Operands are unsigned integer literals accepted by [`parse_unsigned`]. A
/// single literal expression evaluates to that literal, replacing the current
/// value. When the left operand is omitted from a binary expression,
/// `current_value` is used as `operand1`. Unary NOT can omit its operand to apply
/// to `current_value`, and a bit range mask can omit its operand before `[...]`
/// to mask `current_value`. Bit range masks accept either half-open `start:end`
/// ranges or a single bit index, which is treated as `index:index + 1`.
/// Returns an error for malformed expressions or operands that cannot fit in a
/// `u64` or the active integer width.
pub fn parse_expr(
    width: IntegerWidth,
    current_value: u64,
    input: &str,
) -> Result<ParsedExpression, String> {
    for parser in [
        parse_literal_expr,
        parse_binary_expr,
        parse_unary_not_expr,
        parse_bit_range_mask_expr,
    ] {
        match parser(width, current_value, input) {
            Ok(expression) => return Ok(expression),
            Err(e) if is_parse_failure(&e) => {}
            Err(e) => return Err(e),
        }
    }

    Err(format!("Malformed expression '{}'", input))
}

fn is_parse_failure(error: &str) -> bool {
    error == "malformed"
}

fn parse_literal_expr(
    width: IntegerWidth,
    _current_value: u64,
    input: &str,
) -> Result<ParsedExpression, String> {
    static LITERAL_EXPR_RE: OnceLock<Regex> = OnceLock::new();
    let re = LITERAL_EXPR_RE.get_or_init(|| {
        Regex::new(concat!(
            r"^",
            r"\s*",
            r"(?<value>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)",
            r"\s*",
            r"$"
        ))
        .unwrap()
    });
    let caps = re.captures(input).ok_or("malformed")?;
    let value = parse_width_unsigned(width, &caps["value"])
        .map_err(|e| format!("Error parsing literal - {}", e))?;

    Ok(ParsedExpression::Literal { value })
}

fn parse_binary_expr(
    width: IntegerWidth,
    current_value: u64,
    input: &str,
) -> Result<ParsedExpression, String> {
    static BINARY_EXPR_RE: OnceLock<Regex> = OnceLock::new();
    let re = BINARY_EXPR_RE.get_or_init(|| {
        Regex::new(concat!(
            r"^",                                                           // Beginning marker
            r"\s*", // Match any preceding whitespace
            r"(?<operand1>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)?", // Capture the first operand
            r"\s*",                                                         // Match any whitespace
            r"(?<operator>\+|-|\*|\/|%|<<|>>|\||\&|\^)",                    // Capture the operator
            r"\s*",                                                         // Match any whitespace
            r"(?<operand2>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)", // Capture the second operand
            r"\s*", // Match any trailing whitespace
            r"$"    // End marker
        ))
        .unwrap()
    });
    let caps = re.captures(input).ok_or("malformed")?;
    let operand1 = match caps.name("operand1") {
        Some(s) => parse_width_unsigned(width, s.as_str())
            .map_err(|e| format!("Error parsing operand 1 - {}", e))?,
        None => current_value,
    };
    let operand2 = parse_width_unsigned(width, &caps["operand2"])
        .map_err(|e| format!("Error parsing operand 2 - {}", e))?;
    let operator = Operator::parse(&caps["operator"])
        .ok_or_else(|| format!("Unsupported operator: {}", &caps["operator"]))?;

    Ok(ParsedExpression::Binary {
        operand1,
        operator,
        operand2,
    })
}

fn parse_unary_not_expr(
    width: IntegerWidth,
    current_value: u64,
    input: &str,
) -> Result<ParsedExpression, String> {
    static UNARY_NOT_EXPR_RE: OnceLock<Regex> = OnceLock::new();
    let re = UNARY_NOT_EXPR_RE.get_or_init(|| {
        Regex::new(concat!(
            r"^",
            r"\s*",
            r"~",
            r"\s*",
            r"(?<operand>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)?",
            r"\s*",
            r"$"
        ))
        .unwrap()
    });
    let caps = re.captures(input).ok_or("malformed")?;
    let operand = match caps.name("operand") {
        Some(s) => parse_width_unsigned(width, s.as_str())
            .map_err(|e| format!("Error parsing unary operand - {}", e))?,
        None => current_value,
    };

    Ok(ParsedExpression::UnaryNot { operand })
}

fn parse_bit_range_mask_expr(
    width: IntegerWidth,
    current_value: u64,
    input: &str,
) -> Result<ParsedExpression, String> {
    static BIT_RANGE_MASK_EXPR_RE: OnceLock<Regex> = OnceLock::new();
    let re = BIT_RANGE_MASK_EXPR_RE.get_or_init(|| {
        Regex::new(concat!(
            r"^",
            r"\s*",
            r"(?<operand>0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][0-1]+|\d+)?",
            r"\s*",
            r"\[",
            r"\s*",
            r"(?<start_bit>\d+)",
            r"\s*",
            r"(?:",
            r":",
            r"\s*",
            r"(?<end_bit>\d+)",
            r"\s*",
            r")?",
            r"\]",
            r"\s*",
            r"$"
        ))
        .unwrap()
    });
    let caps = re.captures(input).ok_or("malformed")?;
    let operand = match caps.name("operand") {
        Some(s) => parse_width_unsigned(width, s.as_str())
            .map_err(|e| format!("Error parsing range operand - {}", e))?,
        None => current_value,
    };
    let start_bit = parse_bit_range_bound(width, &caps["start_bit"], "start")?;
    let end_bit = match caps.name("end_bit") {
        Some(s) => parse_bit_range_bound(width, s.as_str(), "end")?,
        None => {
            let end_bit = start_bit + 1;
            if end_bit > width.bits() {
                return Err(format!("Invalid bit range end: {}", end_bit));
            }
            end_bit
        }
    };

    validate_bit_range(start_bit, end_bit)?;

    Ok(ParsedExpression::BitRangeMask {
        operand,
        start_bit,
        end_bit,
    })
}

fn parse_width_unsigned(width: IntegerWidth, s: &str) -> Result<u64, String> {
    let value = parse_unsigned(s).map_err(|e| e.to_string())?;
    if value > width.max_value() {
        Err(format!(
            "value {} exceeds {}-bit maximum {}",
            value,
            width.bits(),
            width.max_value()
        ))
    } else {
        Ok(value)
    }
}

fn parse_bit_range_bound(width: IntegerWidth, s: &str, name: &str) -> Result<u32, String> {
    let bound = s
        .parse::<u32>()
        .map_err(|e| format!("Error parsing bit range {} - {}", name, e))?;
    if bound > width.bits() {
        return Err(format!("Invalid bit range {}: {}", name, bound));
    }
    Ok(bound)
}

fn validate_bit_range(start_bit: u32, end_bit: u32) -> Result<(), String> {
    if start_bit > end_bit {
        Err(format!(
            "Invalid bit range: start {} is greater than end {}",
            start_bit, end_bit
        ))
    } else {
        Ok(())
    }
}

pub fn compute_expr(width: IntegerWidth, expression: ParsedExpression) -> Result<u64, String> {
    match expression {
        ParsedExpression::Literal { value } => Ok(value & width.mask()),
        ParsedExpression::Binary {
            operand1,
            operator,
            operand2,
        } => compute(width, operand1, operand2, operator),
        ParsedExpression::UnaryNot { operand } => Ok(!operand & width.mask()),
        ParsedExpression::BitRangeMask {
            operand,
            start_bit,
            end_bit,
        } => compute_bit_range_mask(width, operand, start_bit, end_bit),
    }
}

fn compute_bit_range_mask(
    width: IntegerWidth,
    operand: u64,
    start_bit: u32,
    end_bit: u32,
) -> Result<u64, String> {
    validate_bit_range(start_bit, end_bit)?;
    if end_bit > width.bits() {
        return Err(format!("Invalid bit range end: {}", end_bit));
    }

    let width = end_bit - start_bit;
    let mask = if width == 0 {
        0
    } else if width == u64::BITS {
        u64::MAX
    } else {
        ((1_u64 << width) - 1) << start_bit
    };
    Ok(operand & mask)
}

/// Applies a supported arithmetic, bitwise, or shift operator to two operands.
///
/// Addition, subtraction, and multiplication wrap within the active integer
/// width. Division returns an error when the divisor is zero. Left and right
/// shifts return an error when the shift amount is greater than or equal to the
/// active integer width.
pub fn compute(
    width: IntegerWidth,
    operand1: u64,
    operand2: u64,
    operator: Operator,
) -> Result<u64, String> {
    let mask = width.mask();
    match operator {
        Operator::Add => Ok(operand1.wrapping_add(operand2) & mask),
        Operator::Subtract => Ok(operand1.wrapping_sub(operand2) & mask),
        Operator::Multiply => Ok(operand1.wrapping_mul(operand2) & mask),
        Operator::Divide => operand1
            .checked_div(operand2)
            .map(|result| result & mask)
            .ok_or("Division by zero".into()),
        Operator::Modulo => operand1
            .checked_rem(operand2)
            .map(|result| result & mask)
            .ok_or("Modulo by zero".into()),
        Operator::ShiftLeft => {
            if operand2 >= width.bits().into() {
                Err(format!("Invalid left-shift: {}", operand2))
            } else {
                Ok((operand1 << operand2 as u32) & mask)
            }
        }
        Operator::ShiftRight => {
            if operand2 >= width.bits().into() {
                Err(format!("Invalid right-shift: {}", operand2))
            } else {
                Ok((operand1 >> operand2 as u32) & mask)
            }
        }
        Operator::BitOr => Ok((operand1 | operand2) & mask),
        Operator::BitAnd => Ok((operand1 & operand2) & mask),
        Operator::BitXor => Ok((operand1 ^ operand2) & mask),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::IntErrorKind;

    fn parsed_binary(operand1: u64, operand2: u64, operator: Operator) -> ParsedExpression {
        ParsedExpression::Binary {
            operand1,
            operator,
            operand2,
        }
    }

    fn parsed_literal(value: u64) -> ParsedExpression {
        ParsedExpression::Literal { value }
    }

    fn parse_expr(current_value: u64, input: &str) -> Result<ParsedExpression, String> {
        super::parse_expr(IntegerWidth::SixtyFour, current_value, input)
    }

    fn compute(operand1: u64, operand2: u64, operator: Operator) -> Result<u64, String> {
        super::compute(IntegerWidth::SixtyFour, operand1, operand2, operator)
    }

    fn compute_expr(expression: ParsedExpression) -> Result<u64, String> {
        super::compute_expr(IntegerWidth::SixtyFour, expression)
    }

    #[test]
    fn integer_width_helpers_report_bits_masks_and_cycle() {
        assert_eq!(IntegerWidth::Eight.bits(), 8);
        assert_eq!(IntegerWidth::Sixteen.bits(), 16);
        assert_eq!(IntegerWidth::ThirtyTwo.bits(), 32);
        assert_eq!(IntegerWidth::SixtyFour.bits(), 64);

        assert_eq!(IntegerWidth::Eight.mask(), 0xff);
        assert_eq!(IntegerWidth::Sixteen.mask(), 0xffff);
        assert_eq!(IntegerWidth::ThirtyTwo.mask(), 0xffff_ffff);
        assert_eq!(IntegerWidth::SixtyFour.mask(), u64::MAX);

        assert_eq!(IntegerWidth::Eight.prev(), IntegerWidth::SixtyFour);
        assert_eq!(IntegerWidth::SixtyFour.next(), IntegerWidth::Eight);
    }

    #[test]
    fn parse_unsigned_accepts_decimal_literals() {
        assert_eq!(parse_unsigned("0"), Ok(0));
        assert_eq!(parse_unsigned("42"), Ok(42));
        assert_eq!(parse_unsigned("18446744073709551615"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_hex_literals() {
        assert_eq!(parse_unsigned("0x0"), Ok(0));
        assert_eq!(parse_unsigned("0xff"), Ok(255));
        assert_eq!(parse_unsigned("0XFF"), Ok(255));
        assert_eq!(parse_unsigned("0xffffffffffffffff"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_octal_literals() {
        assert_eq!(parse_unsigned("0o0"), Ok(0));
        assert_eq!(parse_unsigned("0o17"), Ok(15));
        assert_eq!(parse_unsigned("0O17"), Ok(15));
        assert_eq!(parse_unsigned("0o1777777777777777777777"), Ok(u64::MAX));
    }

    #[test]
    fn parse_unsigned_accepts_binary_literals() {
        assert_eq!(parse_unsigned("0b0"), Ok(0));
        assert_eq!(parse_unsigned("0b1010"), Ok(10));
        assert_eq!(parse_unsigned("0B1010"), Ok(10));
        assert_eq!(
            parse_unsigned("0b1111111111111111111111111111111111111111111111111111111111111111"),
            Ok(u64::MAX)
        );
    }

    #[test]
    fn parse_unsigned_rejects_invalid_digits_for_base() {
        assert_eq!(
            *parse_unsigned("0xg").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("0o8").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("0b2").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
        assert_eq!(
            *parse_unsigned("12x").unwrap_err().kind(),
            IntErrorKind::InvalidDigit
        );
    }

    #[test]
    fn parse_unsigned_rejects_empty_or_prefix_only_input() {
        assert_eq!(*parse_unsigned("").unwrap_err().kind(), IntErrorKind::Empty);
        assert_eq!(
            *parse_unsigned("0x").unwrap_err().kind(),
            IntErrorKind::Empty
        );
        assert_eq!(
            *parse_unsigned("0o").unwrap_err().kind(),
            IntErrorKind::Empty
        );
        assert_eq!(
            *parse_unsigned("0b").unwrap_err().kind(),
            IntErrorKind::Empty
        );
    }

    #[test]
    fn parse_unsigned_rejects_overflow() {
        assert_eq!(
            *parse_unsigned("18446744073709551616").unwrap_err().kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0x10000000000000000").unwrap_err().kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0o2000000000000000000000")
                .unwrap_err()
                .kind(),
            IntErrorKind::PosOverflow
        );
        assert_eq!(
            *parse_unsigned("0b10000000000000000000000000000000000000000000000000000000000000000")
                .unwrap_err()
                .kind(),
            IntErrorKind::PosOverflow
        );
    }

    #[test]
    fn parse_expr_accepts_explicit_operands_for_all_operators() {
        assert_eq!(
            parse_expr(0, "1 + 2"),
            Ok(parsed_binary(1, 2, Operator::Add))
        );
        assert_eq!(
            parse_expr(0, "3 - 4"),
            Ok(parsed_binary(3, 4, Operator::Subtract))
        );
        assert_eq!(
            parse_expr(0, "5 * 6"),
            Ok(parsed_binary(5, 6, Operator::Multiply))
        );
        assert_eq!(
            parse_expr(0, "7 / 8"),
            Ok(parsed_binary(7, 8, Operator::Divide))
        );
        assert_eq!(
            parse_expr(0, "7 % 3"),
            Ok(parsed_binary(7, 3, Operator::Modulo))
        );
        assert_eq!(
            parse_expr(0, "9 << 10"),
            Ok(parsed_binary(9, 10, Operator::ShiftLeft))
        );
        assert_eq!(
            parse_expr(0, "11 >> 12"),
            Ok(parsed_binary(11, 12, Operator::ShiftRight))
        );
        assert_eq!(
            parse_expr(0, "13 | 14"),
            Ok(parsed_binary(13, 14, Operator::BitOr))
        );
        assert_eq!(
            parse_expr(0, "15 & 16"),
            Ok(parsed_binary(15, 16, Operator::BitAnd))
        );
        assert_eq!(
            parse_expr(0, "17 ^ 18"),
            Ok(parsed_binary(17, 18, Operator::BitXor))
        );
    }

    #[test]
    fn parse_expr_accepts_single_number_expressions() {
        assert_eq!(parse_expr(999, "32"), Ok(parsed_literal(32)));
        assert_eq!(parse_expr(999, " 0xff "), Ok(parsed_literal(255)));
        assert_eq!(parse_expr(999, "\t0o17\n"), Ok(parsed_literal(15)));
        assert_eq!(parse_expr(999, "0b1010"), Ok(parsed_literal(10)));
    }

    #[test]
    fn parse_expr_accepts_supported_literal_bases() {
        assert_eq!(
            parse_expr(0, "0xff + 0X10"),
            Ok(parsed_binary(255, 16, Operator::Add))
        );
        assert_eq!(
            parse_expr(0, "0o17 + 0O10"),
            Ok(parsed_binary(15, 8, Operator::Add))
        );
        assert_eq!(
            parse_expr(0, "0b1010 + 0B11"),
            Ok(parsed_binary(10, 3, Operator::Add))
        );
    }

    #[test]
    fn parse_expr_accepts_surrounding_whitespace() {
        assert_eq!(
            parse_expr(0, " \t 12 \n + \r 34 \t "),
            Ok(parsed_binary(12, 34, Operator::Add))
        );
    }

    #[test]
    fn parse_expr_accepts_compact_syntax() {
        assert_eq!(parse_expr(0, "1+2"), Ok(parsed_binary(1, 2, Operator::Add)));
        assert_eq!(
            parse_expr(0, "3<<4"),
            Ok(parsed_binary(3, 4, Operator::ShiftLeft))
        );
        assert_eq!(
            parse_expr(42, "<<2"),
            Ok(parsed_binary(42, 2, Operator::ShiftLeft))
        );
    }

    #[test]
    fn parse_expr_accepts_extra_whitespace_between_tokens() {
        assert_eq!(
            parse_expr(0, "2     +   2"),
            Ok(parsed_binary(2, 2, Operator::Add))
        );
        assert_eq!(
            parse_expr(0, "8     >>      1"),
            Ok(parsed_binary(8, 1, Operator::ShiftRight))
        );
        assert_eq!(
            parse_expr(42, "     &      0xff"),
            Ok(parsed_binary(42, 255, Operator::BitAnd))
        );
    }

    #[test]
    fn parse_expr_uses_current_value_when_left_operand_is_omitted() {
        assert_eq!(
            parse_expr(42, "+ 5"),
            Ok(parsed_binary(42, 5, Operator::Add))
        );
        assert_eq!(
            parse_expr(42, "<< 2"),
            Ok(parsed_binary(42, 2, Operator::ShiftLeft))
        );
        assert_eq!(
            parse_expr(42, " & 0xff"),
            Ok(parsed_binary(42, 255, Operator::BitAnd))
        );
    }

    #[test]
    fn parse_expr_accepts_unary_not() {
        assert_eq!(
            parse_expr(0, "~5"),
            Ok(ParsedExpression::UnaryNot { operand: 5 })
        );
        assert_eq!(
            parse_expr(0, "~ 0xff"),
            Ok(ParsedExpression::UnaryNot { operand: 255 })
        );
        assert_eq!(
            parse_expr(42, "~"),
            Ok(ParsedExpression::UnaryNot { operand: 42 })
        );
    }

    #[test]
    fn parse_expr_accepts_bit_range_masks() {
        assert_eq!(
            parse_expr(0, "33[0:2]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 33,
                start_bit: 0,
                end_bit: 2,
            })
        );
        assert_eq!(
            parse_expr(0, "0b1011 [ 1 : 3 ]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 0b1011,
                start_bit: 1,
                end_bit: 3,
            })
        );
        assert_eq!(
            parse_expr(0xff00, "[0:8]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 0xff00,
                start_bit: 0,
                end_bit: 8,
            })
        );
        assert_eq!(
            parse_expr(0, "33[4]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 33,
                start_bit: 4,
                end_bit: 5,
            })
        );
        assert_eq!(
            parse_expr(0b1010, "[ 1 ]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 0b1010,
                start_bit: 1,
                end_bit: 2,
            })
        );
    }

    #[test]
    fn parse_expr_rejects_malformed_expressions() {
        assert_eq!(parse_expr(0, "").unwrap_err(), "Malformed expression ''");
        assert_eq!(
            parse_expr(0, "1 +").unwrap_err(),
            "Malformed expression '1 +'"
        );
        assert_eq!(
            parse_expr(0, "1 2 + 3").unwrap_err(),
            "Malformed expression '1 2 + 3'"
        );
        assert_eq!(
            parse_expr(0, "1 ^^ 2").unwrap_err(),
            "Malformed expression '1 ^^ 2'"
        );
        assert_eq!(
            parse_expr(0, "0xg + 1").unwrap_err(),
            "Malformed expression '0xg + 1'"
        );
        assert_eq!(
            parse_expr(0, "33[:2]").unwrap_err(),
            "Malformed expression '33[:2]'"
        );
        assert_eq!(
            parse_expr(0, "33[0:]").unwrap_err(),
            "Malformed expression '33[0:]'"
        );
        assert_eq!(
            parse_expr(0, "33[]").unwrap_err(),
            "Malformed expression '33[]'"
        );
        assert_eq!(
            parse_expr(0, "33[a:2]").unwrap_err(),
            "Malformed expression '33[a:2]'"
        );
    }

    #[test]
    fn parse_expr_rejects_invalid_bit_ranges() {
        assert_eq!(
            parse_expr(0, "33[3:2]").unwrap_err(),
            "Invalid bit range: start 3 is greater than end 2"
        );
        assert_eq!(
            parse_expr(0, "33[0:65]").unwrap_err(),
            "Invalid bit range end: 65"
        );
        assert_eq!(
            parse_expr(0, "33[64]").unwrap_err(),
            "Invalid bit range end: 65"
        );
    }

    #[test]
    fn parse_expr_reports_operand_overflow() {
        assert_eq!(
            parse_expr(0, "18446744073709551616 + 1").unwrap_err(),
            "Error parsing operand 1 - number too large to fit in target type"
        );
        assert_eq!(
            parse_expr(0, "18446744073709551616").unwrap_err(),
            "Error parsing literal - number too large to fit in target type"
        );
        assert_eq!(
            parse_expr(0, "1 + 18446744073709551616").unwrap_err(),
            "Error parsing operand 2 - number too large to fit in target type"
        );
        assert_eq!(
            parse_expr(0, "~18446744073709551616").unwrap_err(),
            "Error parsing unary operand - number too large to fit in target type"
        );
        assert_eq!(
            parse_expr(0, "18446744073709551616[0:1]").unwrap_err(),
            "Error parsing range operand - number too large to fit in target type"
        );
    }

    #[test]
    fn parse_expr_rejects_operands_above_active_width() {
        assert_eq!(
            super::parse_expr(IntegerWidth::Eight, 0, "256").unwrap_err(),
            "Error parsing literal - value 256 exceeds 8-bit maximum 255"
        );
        assert_eq!(
            super::parse_expr(IntegerWidth::Sixteen, 0, "0x10000 + 1").unwrap_err(),
            "Error parsing operand 1 - value 65536 exceeds 16-bit maximum 65535"
        );
        assert_eq!(
            super::parse_expr(IntegerWidth::ThirtyTwo, 0, "1 + 0x100000000").unwrap_err(),
            "Error parsing operand 2 - value 4294967296 exceeds 32-bit maximum 4294967295"
        );
    }

    #[test]
    fn parse_expr_uses_active_width_for_bit_ranges() {
        assert_eq!(
            super::parse_expr(IntegerWidth::Eight, 0, "255[0:8]"),
            Ok(ParsedExpression::BitRangeMask {
                operand: 255,
                start_bit: 0,
                end_bit: 8,
            })
        );
        assert_eq!(
            super::parse_expr(IntegerWidth::Eight, 0, "255[8]").unwrap_err(),
            "Invalid bit range end: 9"
        );
        assert_eq!(
            super::parse_expr(IntegerWidth::Eight, 0, "255[0:9]").unwrap_err(),
            "Invalid bit range end: 9"
        );
    }

    #[test]
    fn compute_adds_subtracts_and_multiplies_with_wrapping_semantics() {
        assert_eq!(compute(40, 2, Operator::Add), Ok(42));
        assert_eq!(compute(u64::MAX, 1, Operator::Add), Ok(0));
        assert_eq!(compute(u64::MAX - 1, 3, Operator::Add), Ok(1));

        assert_eq!(compute(45, 3, Operator::Subtract), Ok(42));
        assert_eq!(compute(0, 1, Operator::Subtract), Ok(u64::MAX));
        assert_eq!(compute(1, 3, Operator::Subtract), Ok(u64::MAX - 1));

        assert_eq!(compute(6, 7, Operator::Multiply), Ok(42));
        assert_eq!(compute(1_u64 << 63, 2, Operator::Multiply), Ok(0));
        assert_eq!(compute(u64::MAX, 2, Operator::Multiply), Ok(u64::MAX - 1));
    }

    #[test]
    fn compute_divides_and_rejects_division_by_zero() {
        assert_eq!(compute(84, 2, Operator::Divide), Ok(42));
        assert_eq!(
            compute(7, 0, Operator::Divide).unwrap_err(),
            "Division by zero"
        );
    }

    #[test]
    fn compute_modulo_and_rejects_modulo_by_zero() {
        assert_eq!(compute(85, 64, Operator::Modulo), Ok(21));
        assert_eq!(
            compute(7, 0, Operator::Modulo).unwrap_err(),
            "Modulo by zero"
        );
    }

    #[test]
    fn compute_shifts_and_rejects_shift_amounts_of_64_or_more() {
        assert_eq!(compute(1, 63, Operator::ShiftLeft), Ok(1_u64 << 63));
        assert_eq!(
            compute(0x8000_0000_0000_0000, 63, Operator::ShiftRight),
            Ok(1)
        );

        assert_eq!(
            compute(1, 64, Operator::ShiftLeft).unwrap_err(),
            "Invalid left-shift: 64"
        );
        assert_eq!(
            compute(1, 64, Operator::ShiftRight).unwrap_err(),
            "Invalid right-shift: 64"
        );
        assert_eq!(
            compute(1, u64::MAX, Operator::ShiftLeft).unwrap_err(),
            format!("Invalid left-shift: {}", u64::MAX)
        );
        assert_eq!(
            compute(1, u64::MAX, Operator::ShiftRight).unwrap_err(),
            format!("Invalid right-shift: {}", u64::MAX)
        );
    }

    #[test]
    fn compute_applies_bitwise_operators() {
        assert_eq!(compute(0b1010, 0b1100, Operator::BitOr), Ok(0b1110));
        assert_eq!(compute(0b1010, 0b1100, Operator::BitAnd), Ok(0b1000));
        assert_eq!(compute(0b1010, 0b1100, Operator::BitXor), Ok(0b0110));
    }

    #[test]
    fn compute_uses_active_width_for_wrapping_and_masks() {
        assert_eq!(
            super::compute(IntegerWidth::Eight, 255, 1, Operator::Add),
            Ok(0)
        );
        assert_eq!(
            super::compute(IntegerWidth::Sixteen, 0, 1, Operator::Subtract),
            Ok(0xffff)
        );
        assert_eq!(
            super::compute(IntegerWidth::ThirtyTwo, 0xffff_ffff, 2, Operator::Multiply),
            Ok(0xffff_fffe)
        );
        assert_eq!(
            super::compute_expr(
                IntegerWidth::Eight,
                ParsedExpression::UnaryNot { operand: 5 }
            ),
            Ok(0xfa)
        );
    }

    #[test]
    fn compute_uses_active_width_for_shift_limits() {
        assert_eq!(
            super::compute(IntegerWidth::Eight, 1, 7, Operator::ShiftLeft),
            Ok(0x80)
        );
        assert_eq!(
            super::compute(IntegerWidth::Eight, 1, 8, Operator::ShiftLeft).unwrap_err(),
            "Invalid left-shift: 8"
        );
        assert_eq!(
            super::compute(IntegerWidth::Sixteen, 1, 16, Operator::ShiftRight).unwrap_err(),
            "Invalid right-shift: 16"
        );
    }

    #[test]
    fn compute_expr_applies_unary_not() {
        assert_eq!(
            compute_expr(ParsedExpression::UnaryNot { operand: 5 }),
            Ok(!5)
        );
    }

    #[test]
    fn compute_expr_returns_literal_values() {
        assert_eq!(
            compute_expr(ParsedExpression::Literal { value: 32 }),
            Ok(32)
        );
    }

    #[test]
    fn compute_expr_applies_bit_range_masks() {
        assert_eq!(
            compute_expr(ParsedExpression::BitRangeMask {
                operand: 33,
                start_bit: 0,
                end_bit: 2,
            }),
            Ok(1)
        );
        assert_eq!(
            compute_expr(ParsedExpression::BitRangeMask {
                operand: 0b1110,
                start_bit: 1,
                end_bit: 3,
            }),
            Ok(0b0110)
        );
        assert_eq!(
            compute_expr(ParsedExpression::BitRangeMask {
                operand: u64::MAX,
                start_bit: 63,
                end_bit: 64,
            }),
            Ok(1_u64 << 63)
        );
        assert_eq!(
            compute_expr(ParsedExpression::BitRangeMask {
                operand: u64::MAX,
                start_bit: 0,
                end_bit: 64,
            }),
            Ok(u64::MAX)
        );
        assert_eq!(
            compute_expr(ParsedExpression::BitRangeMask {
                operand: u64::MAX,
                start_bit: 2,
                end_bit: 2,
            }),
            Ok(0)
        );
        assert_eq!(compute_expr(parse_expr(0, "33[4]").unwrap()), Ok(0));
        assert_eq!(compute_expr(parse_expr(0, "33[5]").unwrap()), Ok(32));
    }

    #[test]
    fn operator_displays_as_expression_symbol() {
        assert_eq!(Operator::Add.to_string(), "+");
        assert_eq!(Operator::Subtract.to_string(), "-");
        assert_eq!(Operator::Multiply.to_string(), "*");
        assert_eq!(Operator::Divide.to_string(), "/");
        assert_eq!(Operator::Modulo.to_string(), "%");
        assert_eq!(Operator::ShiftLeft.to_string(), "<<");
        assert_eq!(Operator::ShiftRight.to_string(), ">>");
        assert_eq!(Operator::BitOr.to_string(), "|");
        assert_eq!(Operator::BitAnd.to_string(), "&");
        assert_eq!(Operator::BitXor.to_string(), "^");
    }
}
