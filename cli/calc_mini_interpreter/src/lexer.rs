use crate::CalcError;
use std::{matches, num::ParseFloatError};

#[derive(Debug)]
pub enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Equals,
}

fn parse_num(s: &str) -> Result<f64, CalcError> {
    s.parse::<f64>()
        .map_err(|e: ParseFloatError| CalcError::Parse {
            value: s.to_string(),
            reason: e.to_string(),
        })
}

fn ends_value(last: Option<&Token>) -> bool {
    matches!(last, Some(Token::Num(_) | Token::Ident(_) | Token::RParen))
}

pub fn tokenize(raw_text: String) -> Result<Vec<Token>, CalcError> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = raw_text.chars().peekable();
    let mut depth: usize = 0;

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '-' && !ends_value(tokens.last()) {
            chars.next();

            if chars.peek() == Some(&'(') {
                tokens.push(Token::Num(-1.0));
                tokens.push(Token::Star);
                continue;
            }

            let mut num_str = String::from("-");

            while let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_digit() {
                    num_str.push(chars.next().unwrap());
                    //? might need to check for . here also
                } else {
                    // return Err(CalcError::Unknown(chars.next().unwrap().to_string()));
                    break;
                }
            }
            tokens.push(Token::Num(parse_num(&num_str)?));
        } else if c == '+' || c == '-' || c == '*' || c == '/' || c == '(' || c == ')' || c == '=' {
            if c == '(' && ends_value(tokens.last()) {
                tokens.push(Token::Star);
            }
            match c {
                '+' => tokens.push(Token::Plus),
                '-' => tokens.push(Token::Minus),
                '*' => tokens.push(Token::Star),
                '/' => tokens.push(Token::Slash),
                '(' => {
                    depth += 1;
                    tokens.push(Token::LParen)
                }
                ')' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| CalcError::Unknown("unmatched ')'".to_string()))?;
                    tokens.push(Token::RParen)
                }
                '=' => tokens.push(Token::Equals),
                _ => return Err(CalcError::Unknown(chars.next().unwrap().to_string())),
            }
            chars.next();
        } else if c.is_ascii_digit() {
            let mut num_str = String::new();
            let mut has_decimal = false;
            while let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_digit() || next_c == '.' {
                    match next_c {
                        '.' => {
                            if !has_decimal {
                                num_str.push(chars.next().unwrap());
                                if chars.peek() == Some(&'.') {
                                    return Err(CalcError::Unknown(
                                        "two decimals following each other found!".to_string(),
                                    ));
                                }
                                has_decimal = true;
                            } else {
                                return Err(CalcError::Unknown("two decimals found!".to_string()));
                            }
                        }
                        _ => num_str.push(chars.next().unwrap()),
                    }
                } else {
                    break;
                }
            }
            tokens.push(Token::Num(parse_num(&num_str)?));
        } else {
            return Err(CalcError::Unknown(chars.next().unwrap().to_string()));
        }
    }

    if depth > 0 {
        return Err(CalcError::Unknown(format!("{depth} unclosed '('")));
    }

    Ok(tokens)
}

//TODO group letters like x, xx into Ident

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets_must_balance() {
        assert!(tokenize("(2+2)".into()).is_ok());
        assert!(tokenize("-2(2-2)".into()).is_ok());
        assert!(tokenize("((2)+(2))".into()).is_ok());

        assert!(tokenize("(2".into()).is_err());
        assert!(tokenize("((2)".into()).is_err());
        assert!(tokenize("2)".into()).is_err());
        assert!(tokenize("(2))".into()).is_err());
    }
}
