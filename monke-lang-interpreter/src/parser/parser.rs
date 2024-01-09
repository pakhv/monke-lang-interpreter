use super::super::error::InterpreterResult;
use super::ast::{
    Expression, Identifier, InfixExpression, IntegerLiteral, LetStatement, PrefixExpression,
    Program, ReturnStatement, Statement,
};
use crate::lexer::{lexer::Lexer, token::Token};
use crate::parser::ast::{ExpressionStatement, ExpressionType};

#[derive(Debug)]
pub struct Parser {
    lexer: Lexer,
    cur_token: Option<Token>,
    peek_token: Option<Token>,
}

type ParsePrefixFn = fn(&mut Parser) -> InterpreterResult<Box<dyn Expression>>;
type ParseInfixFn = fn(&mut Parser, Box<dyn Expression>) -> InterpreterResult<Box<dyn Expression>>;

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let cur_token = lexer.next_token();
        let peek_token = lexer.next_token();

        Parser {
            lexer,
            cur_token,
            peek_token,
        }
    }

    pub fn parse_program(&mut self) -> InterpreterResult<Program> {
        let mut program = Program { statements: vec![] };

        while self.cur_token.is_some() {
            let statement = self.parse_statement()?;
            program.statements.push(statement);

            self.next_token();
        }

        Ok(program)
    }

    fn parse_statement(&mut self) -> InterpreterResult<Box<dyn Statement>> {
        match &self.cur_token {
            Some(token) => match token {
                Token::Let => Ok(self.parse_let_statement()?),
                Token::Return => Ok(self.parse_return_statement()?),
                _ => Ok(self.parse_expression_statement()?),
            },
            None => Err(String::from(
                "unable to parse statement, there is no tokens",
            )),
        }
    }

    fn next_token(&mut self) {
        self.cur_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
    }

    fn parse_let_statement(&mut self) -> InterpreterResult<Box<dyn Statement>> {
        if !self.expect_peek(Token::Ident(String::new())) {
            return Err(String::from(
                "unable to parse let statement, identifier expected",
            ));
        }

        let statement_name = self.cur_token.clone().unwrap();

        if !self.expect_peek(Token::Assign) {
            return Err(String::from(
                "unable to parse let statement, assign token expected",
            ));
        }

        loop {
            self.next_token();

            match &self.cur_token {
                Some(Token::Semicolon) => break,
                Some(_) => (),
                None => {
                    return Err(String::from(
                        "unable to parse let statement, couldn't find end of statement",
                    ))
                }
            }
        }

        Ok(Box::new(LetStatement {
            token: Token::Let,
            name: Identifier {
                token: statement_name,
            },
            value: Box::new(Identifier {
                token: Token::Illegal,
            }),
        }))
    }

    fn parse_return_statement(&mut self) -> InterpreterResult<Box<dyn Statement>> {
        loop {
            self.next_token();

            match &self.cur_token {
                Some(Token::Semicolon) => break,
                Some(_) => (),
                None => {
                    return Err(String::from(
                        "unable to parse let statement, couldn't find end of statement",
                    ))
                }
            }
        }

        Ok(Box::new(ReturnStatement {
            token: Token::Return,
            return_value: Box::new(Identifier {
                token: Token::Illegal,
            }),
        }))
    }

    fn parse_expression_statement(&mut self) -> InterpreterResult<Box<dyn Statement>> {
        let cur_token = self.cur_token.clone().unwrap();
        let statement_expression = self.parse_expression(ExpressionType::Lowest as usize)?;

        if self
            .peek_token
            .as_ref()
            .is_some_and(|t| t == &Token::Semicolon)
        {
            self.next_token();
        }

        Ok(Box::new(ExpressionStatement {
            token: cur_token,
            expression: statement_expression,
        }))
    }

    fn parse_expression(&mut self, precedence: usize) -> InterpreterResult<Box<dyn Expression>> {
        let prefix_fn = self.get_prefix_fn()?;
        let mut left = prefix_fn(self)?;

        while self
            .peek_token
            .as_ref()
            .is_some_and(|t| t != &Token::Semicolon)
            && precedence < get_precedence(&self.peek_token)
        {
            let infix_fn = self.get_infix_fn()?;
            self.next_token();
            left = infix_fn(self, left)?;
        }

        Ok(left)
    }

    fn expect_peek(&mut self, token: Token) -> bool {
        match &self.peek_token {
            Some(t) if t == &token => {
                self.next_token();
                true
            }
            Some(Token::Ident(_)) | Some(Token::Int(_)) => match token {
                Token::Ident(_) | Token::Int(_) => {
                    self.next_token();
                    true
                }
                _ => false,
            },
            None | Some(_) => false,
        }
    }

    fn get_prefix_fn(&self) -> InterpreterResult<ParsePrefixFn> {
        match &self.cur_token {
            Some(t) => match t {
                Token::Ident(_) => Ok(Self::parse_identifier),
                Token::Int(_) => Ok(Self::parse_integer_literal),
                token if token == &Token::Minus || token == &Token::Bang => {
                    Ok(Self::parse_prefix_expression)
                }
                _ => todo!(),
            },
            None => Err(String::from(
                "unable to parse expression, unknown prefix expression type",
            )),
        }
    }

    fn get_infix_fn(&self) -> InterpreterResult<ParseInfixFn> {
        match &self.peek_token {
            Some(t) => match t {
                Token::Plus => Ok(Self::parse_infix_expression),
                Token::Minus => Ok(Self::parse_infix_expression),
                Token::Asterisk => Ok(Self::parse_infix_expression),
                Token::Slash => Ok(Self::parse_infix_expression),
                Token::Lt => Ok(Self::parse_infix_expression),
                Token::Gt => Ok(Self::parse_infix_expression),
                Token::Eq => Ok(Self::parse_infix_expression),
                Token::Ne => Ok(Self::parse_infix_expression),
                _ => todo!(),
            },
            None => Err(String::from(
                "unable to parse expression, unknown prefix expression type",
            )),
        }
    }

    fn parse_identifier(parser: &mut Parser) -> InterpreterResult<Box<dyn Expression>> {
        Ok(Box::new(Identifier {
            token: parser.cur_token.clone().unwrap(),
        }))
    }

    fn parse_integer_literal(parser: &mut Parser) -> InterpreterResult<Box<dyn Expression>> {
        let token = parser.cur_token.clone().unwrap();

        let value = if let Token::Int(ref number_str) = token {
            number_str
                .parse::<i64>()
                .map_err(|_| String::from("unable to parse integer literal, isize cast error"))?
        } else {
            return Err(String::from(
                "unable to parse integer literal, wrong token found",
            ));
        };

        Ok(Box::new(IntegerLiteral { token, value }))
    }

    fn parse_prefix_expression(parser: &mut Parser) -> InterpreterResult<Box<dyn Expression>> {
        let token = parser.cur_token.clone().unwrap();
        parser.next_token();
        let expression = parser.parse_expression(ExpressionType::Prefix as usize)?;

        Ok(Box::new(PrefixExpression {
            token,
            right: expression,
        }))
    }

    fn parse_infix_expression(
        parser: &mut Parser,
        left: Box<dyn Expression>,
    ) -> InterpreterResult<Box<dyn Expression>> {
        let cur_token = parser.cur_token.clone();
        let cur_precedence = get_precedence(&cur_token);

        parser.next_token();
        let right = parser.parse_expression(cur_precedence)?;

        Ok(Box::new(InfixExpression {
            token: cur_token.unwrap(),
            left,
            right,
        }))
    }
}

fn get_precedence(token: &Option<Token>) -> usize {
    let expr_type = match token {
        Some(t) => match t {
            Token::Plus => ExpressionType::Sum,
            Token::Minus => ExpressionType::Sum,
            Token::Asterisk => ExpressionType::Product,
            Token::Slash => ExpressionType::Product,
            Token::Lt => ExpressionType::LessGreater,
            Token::Gt => ExpressionType::LessGreater,
            Token::Eq => ExpressionType::Equals,
            Token::Ne => ExpressionType::Equals,
            _ => ExpressionType::Lowest,
        },
        None => ExpressionType::Lowest,
    };

    expr_type as usize
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::{
        lexer::{lexer::Lexer, token::Token},
        parser::ast::{
            ExpressionStatement, Identifier, InfixExpression, IntegerLiteral, LetStatement, Node,
            PrefixExpression, Program, ReturnStatement,
        },
    };

    #[test]
    fn let_statements_test() {
        let input = r#"let x = 5;
let y = 10;
let foobar = 838383;"#;
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

        if let Err(err) = &program {
            println!("{err}");
        }

        assert!(program.is_ok());
        let program = program.unwrap();

        assert!(program.statements.len() == 3);

        let expected_identifiers = vec![
            Token::Ident(String::from("x")),
            Token::Ident(String::from("y")),
            Token::Ident(String::from("foobar")),
        ];

        for (expected_token, statement) in expected_identifiers.iter().zip(program.statements) {
            let let_statement = statement
                .as_any()
                .downcast_ref::<LetStatement>()
                .expect("expected let statement");

            assert_eq!(let_statement.token, Token::Let);
            assert_eq!(&let_statement.name.token, expected_token);
        }
    }

    #[test]
    fn return_statements_test() {
        let input = r#"
return 5;
return 10;
return 993322;
"#;
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

        if let Err(err) = &program {
            println!("{err}");
        }

        assert!(program.is_ok());
        let program = program.unwrap();

        assert!(program.statements.len() == 3);

        for statement in program.statements {
            let return_statement = statement
                .as_any()
                .downcast_ref::<ReturnStatement>()
                .expect("expected let statement");

            assert_eq!(return_statement.token, Token::Return);
        }
    }

    #[test]
    fn pretty_print_test() {
        let program = Program {
            statements: vec![Box::new(LetStatement {
                token: Token::Let,
                name: Identifier {
                    token: Token::Ident(String::from("myVar")),
                },
                value: Box::new(Identifier {
                    token: Token::Ident(String::from("anotherVar")),
                }),
            })],
        };

        assert_eq!(
            program.pretty_print(),
            String::from("let myVar = anotherVar;")
        );
    }

    #[test]
    fn identifier_expression_test() {
        let input = "foobar;";
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

        if let Err(err) = &program {
            println!("{err}");
        }

        assert!(program.is_ok());
        let program = program.unwrap();

        assert!(program.statements.len() == 1);
        let expression_statement = program
            .statements
            .first()
            .unwrap()
            .as_any()
            .downcast_ref::<ExpressionStatement>()
            .expect("expected expression statement");

        let identifier = expression_statement
            .expression
            .as_any()
            .downcast_ref::<Identifier>()
            .expect("expected identifier expresssion");

        assert_eq!(identifier.token, Token::Ident(String::from("foobar")));
    }

    #[test]
    fn integer_literal_expression_test() {
        let input = "5;";
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

        if let Err(err) = &program {
            println!("{err}");
        }

        assert!(program.is_ok());
        let program = program.unwrap();

        assert!(program.statements.len() == 1);
        let expression_statement = program
            .statements
            .first()
            .unwrap()
            .as_any()
            .downcast_ref::<ExpressionStatement>()
            .expect("expected expression statement");

        let integer_literal = expression_statement
            .expression
            .as_any()
            .downcast_ref::<IntegerLiteral>()
            .expect("expected integer literal expression");

        assert_eq!(integer_literal.token, Token::Int(String::from("5")));
        assert_eq!(integer_literal.value, 5);
    }

    #[test]
    fn prefix_expression_test() {
        let expected_expressions = vec![("!5;", Token::Bang, 5), ("-15;", Token::Minus, 15)];

        for (input, expected_token, expected_number) in expected_expressions {
            let lexer = Lexer::new(String::from(input));
            let mut parser = Parser::new(lexer);

            let program = parser.parse_program();

            if let Err(err) = &program {
                println!("{err}");
            }

            assert!(program.is_ok());
            let program = program.unwrap();

            assert!(program.statements.len() == 1);
            let expression_statement = program
                .statements
                .first()
                .unwrap()
                .as_any()
                .downcast_ref::<ExpressionStatement>()
                .expect("expected expression statement");

            let prefix_expression = expression_statement
                .expression
                .as_any()
                .downcast_ref::<PrefixExpression>()
                .expect("expected prefix expression");

            assert_eq!(prefix_expression.token, expected_token);

            let integer_literal = prefix_expression
                .right
                .as_any()
                .downcast_ref::<IntegerLiteral>()
                .expect("expected integer literal expression");

            assert_eq!(integer_literal.value, expected_number);
        }
    }

    #[test]
    fn infix_expression_test() {
        let expected_expressions = vec![
            ("5 + 5;", 5, Token::Plus, 5),
            ("5 - 5;", 5, Token::Minus, 5),
            ("5 * 5;", 5, Token::Asterisk, 5),
            ("5 / 5;", 5, Token::Slash, 5),
            ("5 > 5;", 5, Token::Gt, 5),
            ("5 < 5;", 5, Token::Lt, 5),
            ("5 == 5;", 5, Token::Eq, 5),
            ("5 != 5;", 5, Token::Ne, 5),
        ];

        for (input, left, expected_token, right) in expected_expressions {
            let lexer = Lexer::new(String::from(input));
            let mut parser = Parser::new(lexer);

            let program = parser.parse_program();

            if let Err(err) = &program {
                println!("{err}");
            }

            assert!(program.is_ok());
            let program = program.unwrap();

            assert!(program.statements.len() == 1);
            let expression_statement = program
                .statements
                .first()
                .unwrap()
                .as_any()
                .downcast_ref::<ExpressionStatement>()
                .expect("expected expression statement");

            let infix_expression = expression_statement
                .expression
                .as_any()
                .downcast_ref::<InfixExpression>()
                .expect("expected infix expression");

            assert_eq!(infix_expression.token, expected_token);

            let left_integer_literal = infix_expression
                .left
                .as_any()
                .downcast_ref::<IntegerLiteral>()
                .expect("expected integer literal expression");
            let right_integer_literal = infix_expression
                .right
                .as_any()
                .downcast_ref::<IntegerLiteral>()
                .expect("expected integer literal expression");

            assert_eq!(left_integer_literal.value, left);
            assert_eq!(right_integer_literal.value, right);
        }
    }

    #[test]
    fn operator_precedence_test() {
        let expected_expressions = vec![
            ("-a * b", "((-a) * b)"),
            ("!-a", "(!(-a))"),
            ("a + b + c", "((a + b) + c)"),
            ("a + b - c", "((a + b) - c)"),
            ("a * b * c", "((a * b) * c)"),
            ("a * b / c", "((a * b) / c)"),
            ("a + b / c", "(a + (b / c))"),
            ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
            ("3 + 4; -5 * 5", "(3 + 4)((-5) * 5)"),
            ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
            ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            ),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            ),
        ];
        for (input, expected) in expected_expressions {
            let lexer = Lexer::new(String::from(input));
            let mut parser = Parser::new(lexer);

            let program = parser.parse_program();

            if let Err(err) = &program {
                println!("{err}");
            }

            assert!(program.is_ok());
            let program = program.unwrap();

            assert_eq!(program.pretty_print(), expected);
        }
    }
}
