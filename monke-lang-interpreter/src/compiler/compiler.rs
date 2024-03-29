use std::{rc::Rc, usize};

use crate::{
    code::code::{make, Instructions, OpCodeType},
    evaluator::types::{Integer, Object},
    lexer::token::Token,
    parser::ast::{Expression, Program, Statement},
    result::InterpreterResult,
};

#[derive(Debug, Clone)]
struct EmittedInstruction {
    op_code: OpCodeType,
    position: usize,
}

#[derive(Debug)]
pub struct Compiler {
    pub instructions: Instructions,
    pub constants: Vec<Object>,
    last_instruction: Option<EmittedInstruction>,
    prev_instruction: Option<EmittedInstruction>,
}

#[derive(Debug)]
pub struct ByteCode {
    pub instructions: Instructions,
    pub constants: Vec<Object>,
}

impl Compiler {
    const KEKL_VALUE: i32 = 9999;

    pub fn new() -> Self {
        Compiler {
            constants: vec![],
            instructions: Instructions(vec![]),
            last_instruction: None,
            prev_instruction: None,
        }
    }

    pub fn compile(&mut self, program: Program) -> InterpreterResult<()> {
        match program {
            Program::Statements(statements) => {
                for statement in statements {
                    self.compile(statement.into())?;
                }

                Ok(())
            }
            Program::Statement(statement) => match statement.as_ref() {
                Statement::Let(_) => todo!(),
                Statement::Return(_) => todo!(),
                Statement::Expression(expression_statement) => {
                    self.compile(Rc::clone(&expression_statement.expression).into())?;
                    self.emit(OpCodeType::Pop, vec![]);

                    Ok(())
                }
                Statement::Block(block) => {
                    for statement in &block.statements {
                        self.compile(Rc::clone(statement).into())?;
                    }

                    Ok(())
                }
            },
            Program::Expression(expression) => match expression.as_ref() {
                Expression::Identifier(_) => todo!(),
                Expression::IntegerLiteral(int_expression) => {
                    let int = Object::Integer(Integer {
                        value: int_expression.value,
                    });
                    let operand = self.add_constant(int);
                    self.emit(OpCodeType::Constant, vec![operand as i32]);

                    Ok(())
                }
                Expression::StringLiteral(_) => todo!(),
                Expression::Prefix(prefix) => {
                    self.compile(Rc::clone(&prefix.right).into())?;

                    match &prefix.token {
                        Token::Bang => self.emit(OpCodeType::Bang, vec![]),
                        Token::Minus => self.emit(OpCodeType::Minus, vec![]),
                        actual => Err(format!("couldn't compile prefix expression, bang or minus operators expected, but got {actual}"))?,
                    };

                    Ok(())
                }
                Expression::Infix(infix_expression) => {
                    if infix_expression.token == Token::Lt {
                        self.compile(Rc::clone(&infix_expression.right).into())?;
                        self.compile(Rc::clone(&infix_expression.left).into())?;
                        self.emit(OpCodeType::GreaterThan, vec![]);

                        return Ok(());
                    }

                    self.compile(Rc::clone(&infix_expression.left).into())?;
                    self.compile(Rc::clone(&infix_expression.right).into())?;

                    match infix_expression.token {
                        Token::Plus => self.emit(OpCodeType::Add, vec![]),
                        Token::Minus => self.emit(OpCodeType::Sub, vec![]),
                        Token::Asterisk => self.emit(OpCodeType::Mul, vec![]),
                        Token::Slash => self.emit(OpCodeType::Div, vec![]),
                        Token::Gt => self.emit(OpCodeType::GreaterThan, vec![]),
                        Token::Eq => self.emit(OpCodeType::Equal, vec![]),
                        Token::Ne => self.emit(OpCodeType::NotEqual, vec![]),
                        _ => todo!(),
                    };

                    Ok(())
                }
                Expression::Boolean(boolean_expr) => match boolean_expr.value {
                    true => {
                        self.emit(OpCodeType::True, vec![]);
                        Ok(())
                    }
                    false => {
                        self.emit(OpCodeType::False, vec![]);
                        Ok(())
                    }
                },
                Expression::If(if_expression) => {
                    self.compile(Rc::clone(&if_expression.condition).into())?;
                    let jump_not_truthy_pos =
                        self.emit(OpCodeType::JumpNotTruthy, vec![Self::KEKL_VALUE]);

                    self.compile(Rc::clone(&if_expression.consequence).into())?;

                    if self.last_instruction_is_pop() {
                        self.remove_last_pop()?;
                    }

                    match &if_expression.alternative {
                        Some(alternative) => {
                            let jump_pos = self.emit(OpCodeType::Jump, vec![Self::KEKL_VALUE]);

                            let after_consequence_pos = self.instructions.len() as i32;
                            self.change_operand(jump_not_truthy_pos, after_consequence_pos)?;

                            self.compile(Rc::clone(alternative).into())?;

                            if self.last_instruction_is_pop() {
                                self.remove_last_pop()?;
                            }

                            let after_alternative_pos = self.instructions.len() as i32;
                            self.change_operand(jump_pos, after_alternative_pos)?;
                        }
                        None => {
                            let after_consequence_pos = self.instructions.len() as i32;
                            self.change_operand(jump_not_truthy_pos, after_consequence_pos)?;
                        }
                    }

                    Ok(())
                }
                Expression::FunctionLiteral(_) => todo!(),
                Expression::Call(_) => todo!(),
                Expression::ArrayLiteral(_) => todo!(),
                Expression::IndexExpression(_) => todo!(),
                Expression::HashLiteral(_) => todo!(),
            },
        }
    }

    pub fn byte_code(&self) -> ByteCode {
        ByteCode {
            constants: self.constants.clone(),
            instructions: self.instructions.clone(),
        }
    }

    fn add_constant(&mut self, obj: Object) -> usize {
        self.constants.push(obj);
        self.constants.len() - 1
    }

    fn emit(&mut self, op: OpCodeType, operands: Vec<i32>) -> usize {
        let instructions = make(op.clone(), operands);
        let pos = self.add_instructions(instructions);

        self.set_last_instruction(op, pos);

        pos
    }

    fn add_instructions(&mut self, instructions: Instructions) -> usize {
        let new_instruction_position = self.instructions.len();

        for byte in instructions {
            self.instructions.0.push(byte);
        }

        new_instruction_position
    }

    fn set_last_instruction(&mut self, op: OpCodeType, pos: usize) {
        let prev = self.last_instruction.clone();
        let last = Some(EmittedInstruction {
            op_code: op,
            position: pos,
        });

        self.prev_instruction = prev;
        self.last_instruction = last;
    }

    fn last_instruction_is_pop(&self) -> bool {
        match &self.last_instruction {
            Some(instruction) => match instruction.op_code {
                OpCodeType::Pop => true,
                _ => false,
            },
            None => false,
        }
    }

    fn remove_last_pop(&mut self) -> InterpreterResult<()> {
        match &self.last_instruction {
            Some(EmittedInstruction {
                op_code: _,
                position,
            }) => {
                self.instructions = self
                    .instructions
                    .0
                    .get(..*position)
                    .ok_or(String::from("couldn't compile, failed to remove last pop"))?
                    .into();
                self.last_instruction = self.prev_instruction.clone();

                Ok(())
            }
            None => Err(String::from("couldn't compile, failed to remove last pop")),
        }
    }

    fn replace_instructions(
        &mut self,
        pos: usize,
        new_instructions: Instructions,
    ) -> InterpreterResult<()> {
        for idx in 0..new_instructions.len() {
            match (self.instructions.get(pos + idx), new_instructions.get(idx)) {
                (Some(_), Some(_)) => {
                    self.instructions.0[pos + idx] = new_instructions[idx];
                }
                _ => Err(String::from(
                    "couldn't compile, failed to replace intructions",
                ))?,
            }
        }

        Ok(())
    }

    fn change_operand(&mut self, pos: usize, operand: i32) -> InterpreterResult<()> {
        if let None = self.instructions.get(pos) {
            return Err(String::from("couldn't compile, failed change operand"));
        }

        let op: OpCodeType = self.instructions[pos]
            .try_into()
            .map_err(|_| String::from("couldn't compile, failed change operand"))?;
        let new_instructions = make(op, vec![operand]);

        self.replace_instructions(pos, new_instructions)
    }
}

#[cfg(test)]
mod test {
    use core::panic;

    use crate::{
        code::code::{make, Instructions, OpCodeType},
        compiler::compiler::Compiler,
        evaluator::types::Object,
        lexer::lexer::Lexer,
        parser::parser::Parser,
    };

    use super::ByteCode;

    trait ConstTest {
        fn test(&self, actual: &Object);
    }

    impl ConstTest for i64 {
        fn test(&self, actual: &Object) {
            match actual {
                Object::Integer(int) => assert_eq!(int.value, *self),
                not_int => panic!("integer expected, got {not_int}"),
            }
        }
    }

    struct TestCase<T>
    where
        T: ConstTest,
    {
        input: String,
        expected_constants: Vec<T>,
        expected_instructions: Vec<Instructions>,
    }

    fn run_compiler_tests<T>(cases: Vec<TestCase<T>>)
    where
        T: ConstTest,
    {
        for case in cases {
            let lexer = Lexer::new(case.input.clone());
            let mut parser = Parser::new(lexer);

            let program = parser.parse_program();

            if let Err(err) = &program {
                println!("{err}");
            }

            assert!(program.is_ok());
            let program = program.unwrap();

            let mut compiler = Compiler::new();

            if let Err(err) = compiler.compile(program) {
                panic!("{err}");
            }

            let byte_code = compiler.byte_code();

            test_instructions(&byte_code, &case);
            test_constants(&byte_code, &case);
        }
    }

    fn test_constants<T>(byte_code: &ByteCode, expected: &TestCase<T>)
    where
        T: ConstTest,
    {
        assert_eq!(byte_code.constants.len(), expected.expected_constants.len());

        for (idx, constant) in expected.expected_constants.iter().enumerate() {
            constant.test(&byte_code.constants[idx]);
        }
    }

    fn test_instructions<T>(byte_code: &ByteCode, expected: &TestCase<T>)
    where
        T: ConstTest,
    {
        let instructions = expected
            .expected_instructions
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(
            Instructions(instructions).to_string(),
            *byte_code.instructions.to_string()
        );
    }

    #[test]
    fn integer_arithmetic_test() {
        let expected = vec![
            TestCase {
                input: String::from("1 + 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Add, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1; 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Pop, vec![]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 - 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Sub, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 * 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Mul, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("2 / 1"),
                expected_constants: vec![2, 1],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Div, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("-1"),
                expected_constants: vec![1],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Minus, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
        ];

        run_compiler_tests(expected);
    }

    #[test]
    fn boolean_expression_test() {
        let expected: Vec<TestCase<i64>> = vec![
            TestCase {
                input: String::from("true"),
                expected_constants: vec![],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("false"),
                expected_constants: vec![],
                expected_instructions: vec![
                    make(OpCodeType::False, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 > 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::GreaterThan, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 < 2"),
                expected_constants: vec![2, 1],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::GreaterThan, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 == 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Equal, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("1 != 2"),
                expected_constants: vec![1, 2],
                expected_instructions: vec![
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::NotEqual, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("true == false"),
                expected_constants: vec![],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::False, vec![]),
                    make(OpCodeType::Equal, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("true != false"),
                expected_constants: vec![],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::False, vec![]),
                    make(OpCodeType::NotEqual, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("!true"),
                expected_constants: vec![],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::Bang, vec![]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
        ];

        run_compiler_tests(expected);
    }

    #[test]
    fn conditionals_test() {
        let expected: Vec<TestCase<i64>> = vec![
            TestCase {
                input: String::from("if (true) { 10 }; 3333;"),
                expected_constants: vec![10, 3333],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::JumpNotTruthy, vec![7]),
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Pop, vec![]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
            TestCase {
                input: String::from("if (true) { 10 } else { 20 }; 3333;"),
                expected_constants: vec![10, 20, 3333],
                expected_instructions: vec![
                    make(OpCodeType::True, vec![]),
                    make(OpCodeType::JumpNotTruthy, vec![10]),
                    make(OpCodeType::Constant, vec![0]),
                    make(OpCodeType::Jump, vec![13]),
                    make(OpCodeType::Constant, vec![1]),
                    make(OpCodeType::Pop, vec![]),
                    make(OpCodeType::Constant, vec![2]),
                    make(OpCodeType::Pop, vec![]),
                ],
            },
        ];

        run_compiler_tests(expected);
    }
}
