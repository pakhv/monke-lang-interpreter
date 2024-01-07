use crate::lexer::{lexer::Lexer, token::Token};

use super::super::error::InterpreterError;
use super::ast::{Expression, LetStatement, Program, Statement};

#[derive(Debug)]
pub struct Parser {
    lexer: Lexer,
    cur_token: Option<Token>,
    peek_token: Option<Token>,
}

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

    pub fn parse_program(&mut self) -> InterpreterError<Program> {
        let mut program = Program { statements: vec![] };

        while self.cur_token.is_some() {
            let statement = self.parse_statement()?;
            program.statements.push(statement);

            self.next_token();
        }

        Ok(program)
    }

    fn parse_statement(&mut self) -> InterpreterError<Box<dyn Statement>> {
        match &self.cur_token {
            Some(token) => match token {
                Token::Let => Ok(self.parse_let_statement()?),
                _ => Err(String::from("unable to determine statement type")),
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

    fn parse_let_statement(&mut self) -> InterpreterError<Box<dyn Statement>> {
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
                None => return Err(String::from("couldn't find end of statement")),
            }
        }

        Ok(Box::new(LetStatement {
            token: Token::Let,
            name: statement_name,
            value: Expression {},
        }))
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
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::{
        lexer::{lexer::Lexer, token::Token},
        parser::ast::LetStatement,
    };

    #[test]
    fn test_let_statements() {
        let input = r#"let x = 5;
let y = 10;
let foobar = 838383;"#;
        let lexer = Lexer::new(String::from(input));
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program();

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

            assert!(let_statement_valid(let_statement, expected_token));
        }
    }

    fn let_statement_valid(statement: &LetStatement, expected_token: &Token) -> bool {
        if Token::Let != statement.token {
            return false;
        }

        if &statement.name != expected_token {
            return false;
        }

        true
    }
}