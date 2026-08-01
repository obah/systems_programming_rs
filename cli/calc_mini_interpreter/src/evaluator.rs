use crate::CalcError;
use crate::parser::{Expr, Op};
use std::collections::HashMap;

pub type Env = HashMap<String, f64>;

pub fn eval(expr: &Expr, env: &mut Env) -> Result<f64, CalcError> {
    match expr {
        Expr::Num(n) => Ok(*n),

        Expr::Var(name) => env
            .get(name)
            .copied()
            .ok_or_else(|| CalcError::Unknown(format!("undefined variable '{name}'"))),

        Expr::Neg(inner) => Ok(-eval(inner, env)?),

        Expr::BinOp { op, lhs, rhs } => {
            let l = eval(lhs, env)?;
            let r = eval(rhs, env)?;

            match op {
                Op::Add => Ok(l + r),
                Op::Sub => Ok(l - r),
                Op::Mul => Ok(l * r),
                Op::Div if r == 0.0 => Err(CalcError::Unknown("division by zero".to_string())),
                Op::Div => Ok(l / r),
            }
        }

        Expr::Assign { name, value } => {
            let v = eval(value, env)?;
            env.insert(name.clone(), v);
            Ok(v) // `x = 5` is itself worth 5, so `y = x = 5` would work
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn run(src: &str, env: &mut Env) -> Result<f64, CalcError> {
        eval(&parse(&tokenize(src.to_string())?)?, env)
    }

    #[test]
    fn evaluates_the_example() {
        let mut env = Env::new();
        assert_eq!(run("1- 2+3 * (-9/3)", &mut env).unwrap(), -10.0);
    }

    #[test]
    fn errors_dont_produce_values() {
        let mut env = Env::new();
        assert!(run("1/0", &mut env).is_err());
        assert!(run("1/(2-2)", &mut env).is_err());

        assert!(eval(&Expr::Var("x".to_string()), &mut env).is_err());

        let assign = Expr::Assign {
            name: "x".to_string(),
            value: Box::new(Expr::Num(4.0)),
        };
        assert_eq!(eval(&assign, &mut env).unwrap(), 4.0);
        assert_eq!(eval(&Expr::Var("x".to_string()), &mut env).unwrap(), 4.0);
    }
}
