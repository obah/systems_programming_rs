use crate::CalcError;
use crate::lexer::Token;
use std::iter::Peekable;
use std::slice::Iter;

#[derive(Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    BinOp {
        op: Op,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Neg(Box<Expr>),
    Assign {
        name: String,
        value: Box<Expr>,
    },
}

type Tokens<'a> = Peekable<Iter<'a, Token>>;

pub fn parse(tokens: &[Token]) -> Result<Expr, CalcError> {
    let mut it = tokens.iter().peekable();
    let ast = expr(&mut it)?;

    match it.next() {
        None => Ok(ast),
        Some(t) => Err(CalcError::Unknown(format!("trailing token {t:?}"))),
    }
}

fn expr(it: &mut Tokens) -> Result<Expr, CalcError> {
    let mut lhs = term(it)?;

    loop {
        let op = match it.peek() {
            Some(Token::Plus) => Op::Add,
            Some(Token::Minus) => Op::Sub,
            _ => return Ok(lhs),
        };
        it.next();

        let rhs = term(it)?;
        lhs = Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }
}

fn term(it: &mut Tokens) -> Result<Expr, CalcError> {
    let mut lhs = factor(it)?;

    loop {
        let op = match it.peek() {
            Some(Token::Star) => Op::Mul,
            Some(Token::Slash) => Op::Div,
            _ => return Ok(lhs),
        };
        it.next();

        let rhs = factor(it)?;
        lhs = Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }
}

fn factor(it: &mut Tokens) -> Result<Expr, CalcError> {
    match it.next() {
        Some(Token::Num(n)) => Ok(Expr::Num(*n)),
        Some(Token::Ident(name)) => Ok(Expr::Var(name.clone())),
        Some(Token::Minus) => Ok(Expr::Neg(Box::new(factor(it)?))),
        Some(Token::LParen) => {
            let inner = expr(it)?;
            match it.next() {
                Some(Token::RParen) => Ok(inner),
                _ => Err(CalcError::Unknown("expected ')'".to_string())),
            }
        }
        other => Err(CalcError::Unknown(format!(
            "expected a value, got {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn precedence_and_left_assoc() {
        let ast = parse(&tokenize("1- 2+3 * (-9/3)".into()).unwrap()).unwrap();

        assert_eq!(
            ast,
            Expr::BinOp {
                op: Op::Add,
                lhs: Box::new(Expr::BinOp {
                    op: Op::Sub,
                    lhs: Box::new(Expr::Num(1.0)),
                    rhs: Box::new(Expr::Num(2.0)),
                }),
                rhs: Box::new(Expr::BinOp {
                    op: Op::Mul,
                    lhs: Box::new(Expr::Num(3.0)),
                    rhs: Box::new(Expr::BinOp {
                        op: Op::Div,
                        lhs: Box::new(Expr::Num(-9.0)),
                        rhs: Box::new(Expr::Num(3.0)),
                    }),
                }),
            }
        );
    }
}
