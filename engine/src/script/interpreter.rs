use std::collections::HashMap;
use std::sync::Arc;

use super::parser::{parse, BinaryOp, Expr, FunctionDecl, LogicalOp, Stmt, UnaryOp};
use super::ScriptError;

/// A script value. Deliberately plain, owned data (no `Rc`/`RefCell`) so an
/// `Interpreter` — and anything that embeds one, like `ScriptBehavior` — is
/// naturally `Send + Sync` with no extra work.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Str(String),
    Nil,
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Nil => write!(f, "nil"),
        }
    }
}

/// Bridges the interpreter to whatever it's embedded in. Any function call a
/// script makes that isn't a user-defined function or a built-in (`sin`,
/// `cos`, ...) is forwarded here by name — `engine::script` itself has no
/// idea what "log" or "move_by" mean, only the embedder (`engine::behavior`)
/// does.
pub trait Host {
    fn call_native(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError>;
}

/// A `Host` that rejects every native call — used while evaluating
/// top-level `let` initializers (before any real `Host` exists) and by
/// tools, like the standalone script editor, that only need to type-check a
/// script rather than run it against a game.
pub struct NoHost;

impl Host for NoHost {
    fn call_native(&mut self, name: &str, _args: &[Value]) -> Result<Value, ScriptError> {
        Err(ScriptError {
            message: format!("'{name}' is a host function and isn't available here"),
            line: 0,
        })
    }
}

/// What a block of statements did: ran to completion, or hit a `return`.
enum Flow {
    Normal,
    Return(Value),
}

type Scope = HashMap<String, Value>;

/// A compiled script: its top-level function declarations plus the
/// persistent global variables created by its top-level `let` bindings
/// (a script's "fields," surviving across separate `call`s).
pub struct Interpreter {
    functions: HashMap<String, Arc<FunctionDecl>>,
    globals: HashMap<String, Value>,
}

impl Interpreter {
    /// Parses and runs top-level `let`/`fn` statements once. Top-level `let`
    /// initializers can't call host functions (there's no `Host` yet at
    /// compile time) — keep them to constants/built-in math.
    pub fn compile(source: &str) -> Result<Self, Vec<ScriptError>> {
        let statements = parse(source)?;
        let mut interpreter = Interpreter { functions: HashMap::new(), globals: HashMap::new() };
        let mut host = NoHost;
        for stmt in statements {
            match stmt {
                Stmt::FnDecl(decl) => {
                    interpreter.functions.insert(decl.name.clone(), Arc::new(decl));
                }
                Stmt::Let(name, expr) => {
                    let mut scopes = Vec::new();
                    let value = interpreter
                        .eval_expr(&expr, &mut scopes, &mut host)
                        .map_err(|e| vec![e])?;
                    interpreter.globals.insert(name, value);
                }
                _ => {
                    return Err(vec![ScriptError {
                        message: "only 'let' and 'fn' declarations are allowed at the top level".to_string(),
                        line: 0,
                    }]);
                }
            }
        }
        Ok(interpreter)
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Calls a top-level function by name with a fresh call frame (params +
    /// locals only — no closures over any other function's state). Calling
    /// a name that isn't defined is a no-op returning `Nil`, so hooks like
    /// `update`/`interact` are optional: a script just omits the ones it
    /// doesn't need.
    pub fn call(&mut self, name: &str, args: &[Value], host: &mut dyn Host) -> Result<Value, ScriptError> {
        let Some(decl) = self.functions.get(name).cloned() else {
            return Ok(Value::Nil);
        };
        if decl.params.len() != args.len() {
            return Err(ScriptError {
                message: format!(
                    "function '{name}' expects {} argument(s), got {}",
                    decl.params.len(),
                    args.len()
                ),
                line: 0,
            });
        }
        let mut scope = Scope::new();
        for (param, arg) in decl.params.iter().zip(args) {
            scope.insert(param.clone(), arg.clone());
        }
        let mut scopes = vec![scope];
        match self.exec_block(&decl.body, &mut scopes, host)? {
            Flow::Return(value) => Ok(value),
            Flow::Normal => Ok(Value::Nil),
        }
    }

    fn exec_block(&mut self, statements: &[Stmt], scopes: &mut Vec<Scope>, host: &mut dyn Host) -> Result<Flow, ScriptError> {
        for stmt in statements {
            match self.exec_stmt(stmt, scopes, host)? {
                Flow::Normal => {}
                flow @ Flow::Return(_) => return Ok(flow),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, scopes: &mut Vec<Scope>, host: &mut dyn Host) -> Result<Flow, ScriptError> {
        match stmt {
            Stmt::Let(name, expr) => {
                let value = self.eval_expr(expr, scopes, host)?;
                scopes.last_mut().expect("call frame always has a scope").insert(name.clone(), value);
                Ok(Flow::Normal)
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr, scopes, host)?;
                Ok(Flow::Normal)
            }
            Stmt::If(condition, then_branch, else_branch) => {
                if self.eval_expr(condition, scopes, host)?.truthy() {
                    self.exec_scoped(then_branch, scopes, host)
                } else if let Some(else_branch) = else_branch {
                    self.exec_scoped(else_branch, scopes, host)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::While(condition, body) => {
                // A generous but finite cap — a script bug shouldn't be able
                // to hang the whole game in an infinite loop.
                const MAX_ITERATIONS: u32 = 1_000_000;
                let mut iterations = 0u32;
                while self.eval_expr(condition, scopes, host)?.truthy() {
                    match self.exec_scoped(body, scopes, host)? {
                        Flow::Normal => {}
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                    iterations += 1;
                    if iterations > MAX_ITERATIONS {
                        return Err(ScriptError {
                            message: "while loop exceeded 1,000,000 iterations (likely infinite)".to_string(),
                            line: 0,
                        });
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(expr) => self.eval_expr(expr, scopes, host)?,
                    None => Value::Nil,
                };
                Ok(Flow::Return(value))
            }
            // Only top-level `fn` declarations are registered (in `compile`);
            // encountering one mid-body is a no-op rather than an error, since
            // nothing else in the grammar can currently nest a `fn` here.
            Stmt::FnDecl(_) => Ok(Flow::Normal),
        }
    }

    fn exec_scoped(&mut self, statements: &[Stmt], scopes: &mut Vec<Scope>, host: &mut dyn Host) -> Result<Flow, ScriptError> {
        scopes.push(Scope::new());
        let result = self.exec_block(statements, scopes, host);
        scopes.pop();
        result
    }

    fn lookup(&self, name: &str, scopes: &[Scope]) -> Option<Value> {
        for scope in scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    fn assign(&mut self, name: &str, value: Value, scopes: &mut [Scope]) -> Result<(), ScriptError> {
        for scope in scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        if self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), value);
            return Ok(());
        }
        Err(ScriptError { message: format!("assignment to undefined variable '{name}'"), line: 0 })
    }

    fn eval_expr(&mut self, expr: &Expr, scopes: &mut Vec<Scope>, host: &mut dyn Host) -> Result<Value, ScriptError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::Ident(name) => self
                .lookup(name, scopes)
                .ok_or_else(|| ScriptError { message: format!("undefined variable '{name}'"), line: 0 }),
            Expr::Assign(name, value_expr) => {
                let value = self.eval_expr(value_expr, scopes, host)?;
                self.assign(name, value.clone(), scopes)?;
                Ok(value)
            }
            Expr::Unary(op, operand) => {
                let value = self.eval_expr(operand, scopes, host)?;
                match op {
                    UnaryOp::Neg => {
                        let n = value
                            .as_number()
                            .ok_or_else(|| ScriptError { message: "'-' expects a number".to_string(), line: 0 })?;
                        Ok(Value::Number(-n))
                    }
                    UnaryOp::Not => Ok(Value::Bool(!value.truthy())),
                }
            }
            Expr::Logical(lhs, op, rhs) => {
                let left = self.eval_expr(lhs, scopes, host)?;
                match op {
                    LogicalOp::And => {
                        if !left.truthy() {
                            Ok(left)
                        } else {
                            self.eval_expr(rhs, scopes, host)
                        }
                    }
                    LogicalOp::Or => {
                        if left.truthy() {
                            Ok(left)
                        } else {
                            self.eval_expr(rhs, scopes, host)
                        }
                    }
                }
            }
            Expr::Binary(lhs, op, rhs) => {
                let left = self.eval_expr(lhs, scopes, host)?;
                let right = self.eval_expr(rhs, scopes, host)?;
                eval_binary(*op, left, right)
            }
            Expr::Call(name, arg_exprs) => {
                let mut args = Vec::with_capacity(arg_exprs.len());
                for arg_expr in arg_exprs {
                    args.push(self.eval_expr(arg_expr, scopes, host)?);
                }
                self.call_named(name, &args, host)
            }
        }
    }

    /// Resolution order for a call expression: built-in math function, then
    /// a user-defined script function (a fresh call frame — no access to
    /// the caller's locals), then fall through to the embedder's `Host`.
    fn call_named(&mut self, name: &str, args: &[Value], host: &mut dyn Host) -> Result<Value, ScriptError> {
        if let Some(result) = call_builtin(name, args)? {
            return Ok(result);
        }
        if self.functions.contains_key(name) {
            return self.call(name, args, host);
        }
        host.call_native(name, args)
    }
}

fn numeric_error(op: &str) -> ScriptError {
    ScriptError { message: format!("'{op}' expects numbers"), line: 0 }
}

fn numeric_op(op: &str, left: Value, right: Value, f: impl Fn(f64, f64) -> f64) -> Result<Value, ScriptError> {
    let (Value::Number(a), Value::Number(b)) = (&left, &right) else {
        return Err(numeric_error(op));
    };
    Ok(Value::Number(f(*a, *b)))
}

fn compare(op: &str, left: Value, right: Value, f: impl Fn(f64, f64) -> bool) -> Result<Value, ScriptError> {
    let (Value::Number(a), Value::Number(b)) = (&left, &right) else {
        return Err(numeric_error(op));
    };
    Ok(Value::Bool(f(*a, *b)))
}

fn values_equal(a: &Value, b: &Value) -> bool {
    a == b
}

fn eval_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, ScriptError> {
    match op {
        BinaryOp::Add => match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Str(_), _) | (_, Value::Str(_)) => Ok(Value::Str(format!("{left}{right}"))),
            _ => Err(numeric_error("+")),
        },
        BinaryOp::Sub => numeric_op("-", left, right, |a, b| a - b),
        BinaryOp::Mul => numeric_op("*", left, right, |a, b| a * b),
        BinaryOp::Div => numeric_op("/", left, right, |a, b| a / b),
        BinaryOp::Mod => numeric_op("%", left, right, |a, b| a % b),
        BinaryOp::Eq => Ok(Value::Bool(values_equal(&left, &right))),
        BinaryOp::NotEq => Ok(Value::Bool(!values_equal(&left, &right))),
        BinaryOp::Lt => compare("<", left, right, |a, b| a < b),
        BinaryOp::LtEq => compare("<=", left, right, |a, b| a <= b),
        BinaryOp::Gt => compare(">", left, right, |a, b| a > b),
        BinaryOp::GtEq => compare(">=", left, right, |a, b| a >= b),
    }
}

/// Built-in pure math functions, always available with no `Host` needed.
fn call_builtin(name: &str, args: &[Value]) -> Result<Option<Value>, ScriptError> {
    fn arg(args: &[Value], index: usize, name: &str) -> Result<f64, ScriptError> {
        args.get(index)
            .and_then(Value::as_number)
            .ok_or_else(|| ScriptError { message: format!("'{name}' expects numeric argument(s)"), line: 0 })
    }
    fn check_arity(name: &str, args: &[Value], expected: usize) -> Result<(), ScriptError> {
        if args.len() != expected {
            return Err(ScriptError {
                message: format!("'{name}' expects {expected} argument(s), got {}", args.len()),
                line: 0,
            });
        }
        Ok(())
    }

    let value = match name {
        "sin" => { check_arity(name, args, 1)?; Value::Number(arg(args, 0, name)?.sin()) }
        "cos" => { check_arity(name, args, 1)?; Value::Number(arg(args, 0, name)?.cos()) }
        "abs" => { check_arity(name, args, 1)?; Value::Number(arg(args, 0, name)?.abs()) }
        "sqrt" => { check_arity(name, args, 1)?; Value::Number(arg(args, 0, name)?.sqrt()) }
        "floor" => { check_arity(name, args, 1)?; Value::Number(arg(args, 0, name)?.floor()) }
        "min" => { check_arity(name, args, 2)?; Value::Number(arg(args, 0, name)?.min(arg(args, 1, name)?)) }
        "max" => { check_arity(name, args, 2)?; Value::Number(arg(args, 0, name)?.max(arg(args, 1, name)?)) }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingHost {
        calls: Vec<(String, Vec<Value>)>,
    }

    impl Host for RecordingHost {
        fn call_native(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError> {
            self.calls.push((name.to_string(), args.to_vec()));
            Ok(Value::Nil)
        }
    }

    #[test]
    fn arithmetic_and_precedence() {
        let mut interp = Interpreter::compile("fn calc() { return 2 + 3 * 4 - 1; }").unwrap();
        let mut host = NoHost;
        assert_eq!(interp.call("calc", &[], &mut host).unwrap(), Value::Number(13.0));
    }

    #[test]
    fn if_else_and_comparison() {
        let src = "fn classify(n) { if n > 0 { return \"pos\"; } else { return \"nonpos\"; } }";
        let mut interp = Interpreter::compile(src).unwrap();
        let mut host = NoHost;
        assert_eq!(
            interp.call("classify", &[Value::Number(5.0)], &mut host).unwrap(),
            Value::Str("pos".to_string())
        );
        assert_eq!(
            interp.call("classify", &[Value::Number(-1.0)], &mut host).unwrap(),
            Value::Str("nonpos".to_string())
        );
    }

    #[test]
    fn while_loop_and_globals_persist_across_calls() {
        let src = "let total = 0.0;\nfn accumulate(n) { let i = 0.0; while i < n { total = total + 1.0; i = i + 1.0; } return total; }";
        let mut interp = Interpreter::compile(src).unwrap();
        let mut host = NoHost;
        assert_eq!(interp.call("accumulate", &[Value::Number(3.0)], &mut host).unwrap(), Value::Number(3.0));
        // Calling again should keep accumulating in the persistent global.
        assert_eq!(interp.call("accumulate", &[Value::Number(2.0)], &mut host).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn built_in_math_functions() {
        let mut interp = Interpreter::compile("fn calc() { return sqrt(16.0) + abs(-3.0) + max(1.0, 9.0); }").unwrap();
        let mut host = NoHost;
        assert_eq!(interp.call("calc", &[], &mut host).unwrap(), Value::Number(4.0 + 3.0 + 9.0));
    }

    #[test]
    fn unknown_call_forwards_to_host() {
        let mut interp = Interpreter::compile("fn go() { log(\"hi\", 42.0); }").unwrap();
        let mut host = RecordingHost { calls: Vec::new() };
        interp.call("go", &[], &mut host).unwrap();
        assert_eq!(host.calls.len(), 1);
        assert_eq!(host.calls[0].0, "log");
        assert_eq!(host.calls[0].1, vec![Value::Str("hi".to_string()), Value::Number(42.0)]);
    }

    #[test]
    fn missing_function_call_is_a_noop() {
        let mut interp = Interpreter::compile("fn go() {}").unwrap();
        let mut host = NoHost;
        assert_eq!(interp.call("nonexistent", &[], &mut host).unwrap(), Value::Nil);
    }

    #[test]
    fn string_concatenation() {
        let mut interp = Interpreter::compile("fn greet(name) { return \"hi \" + name; }").unwrap();
        let mut host = NoHost;
        assert_eq!(
            interp.call("greet", &[Value::Str("world".to_string())], &mut host).unwrap(),
            Value::Str("hi world".to_string())
        );
    }

    #[test]
    fn parse_error_reports_line_number() {
        let result = Interpreter::compile("fn broken( {\n  return 1;\n}");
        match result {
            Err(errors) => assert!(!errors.is_empty()),
            Ok(_) => panic!("expected a parse error"),
        }
    }
}
