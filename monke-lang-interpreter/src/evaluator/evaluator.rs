use crate::{
    lexer::token::Token,
    parser::ast::{Expression, Program, Statement},
    result::InterpreterResult,
};

use super::types::{Boolean, Integer, Null, Object};

pub fn eval(program: Program) -> InterpreterResult<Object> {
    match program {
        Program::Statement(statement) => match statement {
            Statement::Expression(expr) => eval(expr.expression.into()),
            _ => todo!(),
        },
        Program::Statements(statements) => Ok(eval_statements(statements)?),
        Program::Expression(expr) => match expr {
            Expression::IntegerLiteral(int) => Ok(Object::Integer(Integer { value: int.value })),
            Expression::Boolean(bool) => Ok(Object::Boolean(Boolean { value: bool.value })),
            Expression::Prefix(prefix) => {
                let right = eval((*prefix.right).into())?;
                Ok(eval_prefix_expression(prefix.token, right)?)
            }
            Expression::Infix(infix) => {
                let left = eval((*infix.left).into())?;
                let right = eval((*infix.right).into())?;

                Ok(eval_infix_expression(infix.token, left, right)?)
            }
            _ => todo!(),
        },
        Program::Expressions(_) => todo!(),
    }
}

fn eval_prefix_expression(token: Token, right: Object) -> InterpreterResult<Object> {
    match token {
        Token::Bang => match right {
            Object::Boolean(bool) => Ok(Object::Boolean(Boolean { value: !bool.value })),
            Object::Null(_) => Ok(Object::Boolean(Boolean { value: true })),
            _ => Ok(Object::Boolean(Boolean { value: false })),
        },
        Token::Minus => match right {
            Object::Integer(int) => Ok(Object::Integer(Integer { value: -int.value })),
            expr => Err(format!(
                "unable to evaluate prefix expression, Integer number must follow Minus token, but got {expr}"
            )),
        },
        t => Err(format!(
            "unable to evaluate prefix expression, ! or - tokens expected, but got {t}",
        )),
    }
}

fn eval_infix_expression(token: Token, left: Object, right: Object) -> InterpreterResult<Object> {
    match (left, right) {
        (Object::Integer(int_left), Object::Integer(int_right)) => match token {
            Token::Plus => Ok(Object::Integer(Integer {
                value: int_left.value + int_right.value,
            })),
            Token::Minus => Ok(Object::Integer(Integer {
                value: int_left.value - int_right.value,
            })),
            Token::Asterisk => Ok(Object::Integer(Integer {
                value: int_left.value * int_right.value,
            })),
            Token::Slash => Ok(Object::Integer(Integer {
                value: int_left.value / int_right.value,
            })),
            Token::Lt => Ok(Object::Boolean(Boolean {
                value: int_left.value < int_right.value,
            })),
            Token::Gt => Ok(Object::Boolean(Boolean {
                value: int_left.value > int_right.value,
            })),
            Token::Eq => Ok(Object::Boolean(Boolean {
                value: int_left.value == int_right.value,
            })),
            Token::Ne => Ok(Object::Boolean(Boolean {
                value: int_left.value != int_right.value,
            })),
            t => Err(format!(
                "unable to evaluate infix expression; +,-,*,/,<,>,==,!= Tokens expected, but got {t}"
            )),
        },
        (Object::Boolean(bool_left),Object::Boolean(bool_right)) => match token {
            Token::Eq => Ok(Object::Boolean(Boolean { value: bool_left.value == bool_right.value })),
            Token::Ne=> Ok(Object::Boolean(Boolean { value: bool_left.value != bool_right.value })),
            t => Err(format!(
                "unable to evaluate infix expression; == or != Tokens expected, but got {t}"
            )),
        }
        (left, right) => Err(format!(
            "unable to evaluate infix expression, Integer numbers expected, but got {left} {right}"
        )),
    }
}

fn eval_statements(statements: Vec<Statement>) -> InterpreterResult<Object> {
    let mut result = Object::Null(Null {});

    for statement in statements {
        result = eval(statement.into())?;
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::{
        evaluator::{evaluator::eval, types::Object},
        lexer::lexer::Lexer,
        parser::parser::Parser,
    };

    fn evaluate_input(input: String) -> Object {
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

        if let Err(err) = &program {
            println!("{err}");
        }

        assert!(program.is_ok());
        let program = program.unwrap();

        let result = eval(program);

        if let Err(err) = &result {
            println!("{err}");
        }

        assert!(result.is_ok());

        result.unwrap()
    }

    #[test]
    fn integer_expression_evaluation_test() {
        let expected = vec![
            ("5", 5),
            ("10", 10),
            ("-5", -5),
            ("-10", -10),
            ("5 + 5 + 5 + 5 - 10", 10),
            ("2 * 2 * 2 * 2 * 2", 32),
            ("-50 + 100 + -50", 0),
            ("5 * 2 + 10", 20),
            ("5 + 2 * 10", 25),
            ("20 + 2 * -10", 0),
            ("50 / 2 * 2 + 10", 60),
            ("2 * (5 + 10)", 30),
            ("3 * 3 * 3 + 10", 37),
            ("3 * (3 * 3) + 10", 37),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", 50),
        ];

        for (input, expected_result) in expected {
            let result = evaluate_input(input.to_string());

            match result {
                Object::Integer(int) => assert_eq!(int.value, expected_result),
                actual => panic!("integer expected, but got {actual}"),
            }
        }
    }

    #[test]
    fn boolean_expression_evaluation_test() {
        let expected = vec![
            ("true", true),
            ("false", false),
            ("1 < 2", true),
            ("1 > 2", false),
            ("1 < 1", false),
            ("1 > 1", false),
            ("1 == 1", true),
            ("1 != 1", false),
            ("1 == 2", false),
            ("1 != 2", true),
            ("true == true", true),
            ("false == false", true),
            ("true == false", false),
            ("true != false", true),
            ("false != true", true),
            ("(1 < 2) == true", true),
            ("(1 < 2) == false", false),
            ("(1 > 2) == true", false),
            ("(1 > 2) == false", true),
        ];

        for (input, expected_result) in expected {
            let result = evaluate_input(input.to_string());

            match result {
                Object::Boolean(bool) => assert_eq!(bool.value, expected_result),
                actual => panic!("integer expected, but got {actual}"),
            }
        }
    }

    #[test]
    fn bang_operator_evaluation_test() {
        let expected = vec![
            ("!true", false),
            ("!false", true),
            ("!5", false),
            ("!!true", true),
            ("!!false", false),
            ("!!5", true),
        ];

        for (input, expected_result) in expected {
            let result = evaluate_input(input.to_string());

            match result {
                Object::Boolean(bool) => assert_eq!(bool.value, expected_result),
                actual => panic!("integer expected, but got {actual}"),
            }
        }
    }
}
