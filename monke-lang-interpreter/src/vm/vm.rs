use std::{array::from_fn, usize};

use crate::{
    code::code::{read_u16, Instructions, OpCodeType},
    compiler::compiler::ByteCode,
    evaluator::types::{Boolean, Integer, Null, Object},
    result::InterpreterResult,
};

const STACK_SIZE: usize = 2048;

#[derive(Debug)]
pub struct Vm {
    constants: Vec<Object>,
    instructions: Instructions,

    stack: [Object; STACK_SIZE],
    sp: usize,
}

impl Vm {
    pub fn new(byte_code: ByteCode) -> Self {
        Vm {
            constants: byte_code.constants,
            instructions: byte_code.instructions,
            stack: from_fn(|_| Object::Null(Null {})),
            sp: 0,
        }
    }

    pub fn stack_top(&self) -> Option<&Object> {
        self.stack.get(self.sp - 1)
    }

    pub fn run(&mut self) -> InterpreterResult<()> {
        let mut ip = 0;

        while ip < self.instructions.len() {
            let op: OpCodeType = (*self
                .instructions
                .get(ip)
                .ok_or(format!("couldn't parse byte code"))?)
            .try_into()?;

            match op {
                OpCodeType::Constant => {
                    let const_idx = read_u16(
                        self.instructions
                            .get(ip + 1..)
                            .ok_or(format!("couldn't parse byte code"))?,
                    );
                    ip += 2;

                    self.push(
                        self.constants
                            .get(const_idx as usize)
                            .ok_or(format!("couldn't parse byte code"))?
                            .clone(),
                    )?;
                }
                op if op == OpCodeType::Add
                    || op == OpCodeType::Sub
                    || op == OpCodeType::Mul
                    || op == OpCodeType::Div =>
                {
                    self.execute_binary_operation(op)?;
                }
                OpCodeType::Pop => {
                    self.pop()?;
                }
                OpCodeType::True => {
                    self.push(Object::Boolean(Boolean { value: true }))?;
                }
                OpCodeType::False => {
                    self.push(Object::Boolean(Boolean { value: false }))?;
                }
                op if op == OpCodeType::GreaterThan
                    || op == OpCodeType::Equal
                    || op == OpCodeType::NotEqual =>
                {
                    self.execute_comparison(op)?;
                }
                OpCodeType::Bang => match self.pop()? {
                    Object::Boolean(bool) => {
                        self.push(Object::Boolean(Boolean { value: !bool.value }))?
                    }
                    _ => self.push(Object::Boolean(Boolean { value: false }))?,
                },
                OpCodeType::Minus => match self.pop()? {
                    Object::Integer(int) => {
                        self.push(Object::Integer(Integer { value: -int.value }))?
                    }
                    actual => Err(format!("unsupported type for negation, got {actual}"))?,
                },
                _ => todo!(),
            }

            ip += 1;
        }

        Ok(())
    }

    pub fn last_popped_stack_elem(&self) -> InterpreterResult<Object> {
        Ok(self
            .stack
            .get(self.sp)
            .ok_or(String::from(
                "couldn't pop from the stack, index is out of bounds",
            ))?
            .clone())
    }

    fn push(&mut self, object: Object) -> InterpreterResult<()> {
        if self.sp >= STACK_SIZE {
            return Err(String::from("stack overflow"));
        }

        self.stack[self.sp] = object;
        self.sp += 1;

        Ok(())
    }

    fn pop(&mut self) -> InterpreterResult<Object> {
        self.sp -= 1;

        Ok(self
            .stack
            .get(self.sp)
            .ok_or(String::from(
                "couldn't pop from the stack, index is out of bounds",
            ))?
            .clone())
    }

    fn execute_binary_operation(&mut self, op: OpCodeType) -> InterpreterResult<()> {
        let right = self.pop()?;
        let left = self.pop()?;

        match (left, right) {
            (Object::Integer(left_int), Object::Integer(right_int)) => match op {
                OpCodeType::Add => self.push(Object::Integer(Integer {
                    value: left_int.value + right_int.value,
                })),
                OpCodeType::Sub => self.push(Object::Integer(Integer {
                    value: left_int.value - right_int.value,
                })),
                OpCodeType::Mul => self.push(Object::Integer(Integer {
                    value: left_int.value * right_int.value,
                })),
                OpCodeType::Div => self.push(Object::Integer(Integer {
                    value: left_int.value / right_int.value,
                })),
                t => Err(format!(
                    "couldn't execute binary operation, wrong operation type - {t}"
                ))?,
            },
            (obj1, obj2) => Err(format!(
                "couldn't execute binary operation: got {obj1} and {obj2}"
            ))?,
        }
    }

    fn execute_comparison(&mut self, op: OpCodeType) -> InterpreterResult<()> {
        let right = self.pop()?;
        let left = self.pop()?;

        match (left, right) {
            (Object::Integer(int1), Object::Integer(int2)) => match op {
                OpCodeType::Equal => self.push(Object::Boolean(Boolean {
                    value: int1.value == int2.value,
                })),
                OpCodeType::NotEqual => self.push(Object::Boolean(Boolean {
                    value: int1.value != int2.value,
                })),
                OpCodeType::GreaterThan => self.push(Object::Boolean(Boolean {
                    value: int1.value > int2.value,
                })),
                op => Err(format!(
                    "couldn't compare two objects, got wrong operator {op}"
                )),
            },
            (Object::Boolean(bool1), Object::Boolean(bool2)) => match op {
                OpCodeType::Equal => self.push(Object::Boolean(Boolean {
                    value: bool1.value == bool2.value,
                })),
                OpCodeType::NotEqual => self.push(Object::Boolean(Boolean {
                    value: bool1.value != bool2.value,
                })),
                OpCodeType::GreaterThan => self.push(Object::Boolean(Boolean {
                    value: bool1.value > bool2.value,
                })),
                op => Err(format!(
                    "couldn't compare two objects, got wrong operator {op}"
                )),
            },
            (actual_left, actual_right) => Err(format!(
                "couldn't compare two objects, got {actual_left} and {actual_right}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compiler::compiler::Compiler, evaluator::types::Object, lexer::lexer::Lexer,
        parser::parser::Parser,
    };

    use super::*;

    struct TestCase<T>
    where
        T: ConstTest,
    {
        input: String,
        expected: T,
    }

    impl ConstTest for i64 {
        fn test(&self, actual: &Object) {
            match actual {
                Object::Integer(int) => assert_eq!(int.value, *self),
                not_int => panic!("integer expected, got {not_int}"),
            }
        }
    }

    impl ConstTest for bool {
        fn test(&self, actual: &Object) {
            match actual {
                Object::Boolean(bool) => assert_eq!(bool.value, *self),
                not_int => panic!("integer expected, got {not_int}"),
            }
        }
    }

    trait ConstTest {
        fn test(&self, actual: &Object);
    }

    fn run_vm_tests<T>(cases: Vec<TestCase<T>>)
    where
        T: ConstTest,
    {
        for case in cases {
            let lexer = Lexer::new(case.input.clone());
            let mut parser = Parser::new(lexer);

            let program = parser.parse_program();

            if let Err(err) = &program {
                panic!("{err}");
            }

            let program = program.unwrap();

            let mut compiler = Compiler::new();

            if let Err(err) = compiler.compile(program) {
                panic!("{err}");
            }

            let byte_code = compiler.byte_code();
            let mut vm = Vm::new(byte_code);

            if let Err(err) = vm.run() {
                panic!("{err}");
            }

            let stack_elem = vm.last_popped_stack_elem();
            assert!(stack_elem.is_ok());
            let stack_elem = stack_elem.unwrap();

            case.expected.test(&stack_elem);
        }
    }

    #[test]
    fn integer_arithmetic_test() {
        let expected = vec![
            TestCase {
                input: String::from("1"),
                expected: 1,
            },
            TestCase {
                input: String::from("2"),
                expected: 2,
            },
            TestCase {
                input: String::from("1 + 2"),
                // todo: fix later
                expected: 3,
            },
            TestCase {
                input: String::from("1 - 2"),
                expected: -1,
            },
            TestCase {
                input: String::from("1 * 2"),
                expected: 2,
            },
            TestCase {
                input: String::from("4 / 2"),
                expected: 2,
            },
            TestCase {
                input: String::from("50 / 2 * 2 + 10 - 5"),
                expected: 55,
            },
            TestCase {
                input: String::from("5 + 5 + 5 + 5 - 10"),
                expected: 10,
            },
            TestCase {
                input: String::from("2 * 2 * 2 * 2 * 2"),
                expected: 32,
            },
            TestCase {
                input: String::from("5 * 2 + 10"),
                expected: 20,
            },
            TestCase {
                input: String::from("5 + 2 * 10"),
                expected: 25,
            },
            TestCase {
                input: String::from("5 * (2 + 10)"),
                expected: 60,
            },
            TestCase {
                input: String::from("-5"),
                expected: -5,
            },
            TestCase {
                input: String::from("-10"),
                expected: -10,
            },
            TestCase {
                input: String::from("-50 + 100 + -50"),
                expected: 0,
            },
            TestCase {
                input: String::from("(5 + 10 * 2 + 15 / 3) * 2 + -10"),
                expected: 50,
            },
        ];

        run_vm_tests(expected);
    }

    #[test]
    fn boolean_expression_test() {
        let expected = vec![
            TestCase {
                input: String::from("true"),
                expected: true,
            },
            TestCase {
                input: String::from("false"),
                expected: false,
            },
            TestCase {
                input: String::from("1 < 2"),
                expected: true,
            },
            TestCase {
                input: String::from("1 > 2"),
                expected: false,
            },
            TestCase {
                input: String::from("1 < 1"),
                expected: false,
            },
            TestCase {
                input: String::from("1 > 1"),
                expected: false,
            },
            TestCase {
                input: String::from("1 == 1"),
                expected: true,
            },
            TestCase {
                input: String::from("1 != 1"),
                expected: false,
            },
            TestCase {
                input: String::from("1 == 2"),
                expected: false,
            },
            TestCase {
                input: String::from("1 != 2"),
                expected: true,
            },
            TestCase {
                input: String::from("true == true"),
                expected: true,
            },
            TestCase {
                input: String::from("false == false"),
                expected: true,
            },
            TestCase {
                input: String::from("true == false"),
                expected: false,
            },
            TestCase {
                input: String::from("true != false"),
                expected: true,
            },
            TestCase {
                input: String::from("false != true"),
                expected: true,
            },
            TestCase {
                input: String::from("(1 < 2) == true"),
                expected: true,
            },
            TestCase {
                input: String::from("(1 < 2) == false"),
                expected: false,
            },
            TestCase {
                input: String::from("(1 > 2) == true"),
                expected: false,
            },
            TestCase {
                input: String::from("(1 > 2) == false"),
                expected: true,
            },
            TestCase {
                input: String::from("!true"),
                expected: false,
            },
            TestCase {
                input: String::from("!false"),
                expected: true,
            },
            TestCase {
                input: String::from("!5"),
                expected: false,
            },
            TestCase {
                input: String::from("!!true"),
                expected: true,
            },
            TestCase {
                input: String::from("!!false"),
                expected: false,
            },
            TestCase {
                input: String::from("!!5"),
                expected: true,
            },
        ];

        run_vm_tests(expected);
    }
}
