# Helix Token System & Architecture Guide

## Overview

Helix is a powerful configuration language and runtime system that processes `.hlx` files through a multi-stage pipeline: **Lexer → Parser → AST → Evaluator**. This guide explains how to enhance and extend the Helix system for developers and AI agents.

## Architecture Overview

```
Input (.hlx file)
       ↓
    Lexer (tokenizes)
       ↓
    Parser (builds AST)
       ↓
    AST (abstract syntax tree)
       ↓
    Evaluator (ops.rs + interpreter)
       ↓
Schema Generation (8+ languages)
       ↓
Output (configuration data + SDKs)
```

## 1. The Lexer (`src/lexer.rs`)

The lexer converts raw text into tokens that the parser can understand.

### Token Types

```rust
pub enum Token {
    // Literals
    String(String),           // "hello" or 'hello'
    Number(f64),             // 42, 3.14
    Bool(bool),              // true, false
    Identifier(String),      // variable names, section names
    
    // Symbols
    Assign,                  // =
    Plus,                    // + (binary operator)
    Minus,                   // - (unary/binary)
    LeftBrace, RightBrace,   // { }
    LeftBracket, RightBracket, // [ ]
    LeftParen, RightParen,   // ( )
    Comma,                   // ,
    Dot,                     // .
    Colon,                   // :
    Semicolon,               // ;
    
    // Special
    Tilde,                   // ~ (generic sections)
    At,                      // @ (operators)
    Variable(String),        // $VAR (global variables)
    Reference(String),       // @env, @date, etc.
    
    // Keywords
    Keyword(Keyword),        // project, agent, workflow, task, etc.
}
```

### Adding New Tokens

To add a new token:

1. **Add to Token enum** in `src/lexer.rs`:
```rust
pub enum Token {
    // ... existing tokens
    NewToken,  // Add your token here
}
```

2. **Add lexing logic** in the `next_token_internal()` function:
```rust
Some('+') => {
    self.advance();
    Token::Plus
}
Some('*') => {
    self.advance();
    Token::Multiply  // New token
}
```

3. **Add precedence** if it's an operator (in `src/parser.rs`):
```rust
fn get_token_precedence(&self, token: &Token) -> Precedence {
    match token {
        Token::Plus => Precedence::Addition,
        Token::Multiply => Precedence::Multiplication,  // New precedence
        // ... other precedences
    }
}
```

## 2. The Parser (`src/parser.rs`)

The parser converts tokens into an Abstract Syntax Tree (AST).

### Parser Flow

```
parse() → loops through tokens
    ├── Token::Keyword → parse_declaration()
    ├── Token::Tilde → parse_generic_declaration()  
    ├── Token::Identifier → parse_generic_declaration()
    └── Token::Eof → done
```

### Declaration Types

```rust
pub enum Declaration {
    Project(ProjectDecl),
    Agent(AgentDecl),
    Workflow(WorkflowDecl),
    Task(TaskDecl),         // Task declarations
    Section(SectionDecl),   // Generic sections
    // ... more specialized declarations
}
```

### Expression Types

```rust
pub enum Expression {
    String(String),
    Number(f64),
    Bool(bool),
    Variable(String),           // $VAR
    AtOperatorCall(String, HashMap<String, Expression>),  // @env[...]
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),  // a + b
    // ... more
}
```

### Adding New Keywords

To add a new keyword declaration (like `database`):

1. **Add to Keyword enum** in `src/lexer.rs`:
```rust
pub enum Keyword {
    // ... existing
    Database,
}
```

2. **Add keyword mapping** in `check_keyword()`:
```rust
"database" => Some(Keyword::Database),
```

3. **Add parsing logic** in `parse_declaration()` in `src/parser.rs`:
```rust
match keyword {
    Keyword::Database => {
        self.advance();
        let (name, block_kind) = self.parse_generic_variations("database".to_string())?;
        // ... parsing logic
        Ok(Declaration::Database(DatabaseDecl { name, properties }))
    }
    // ... other keywords
}
```

### Adding Binary Operators

To add a new binary operator (like `*` for multiplication):

1. **Add token** (see lexer section above)

2. **Add precedence** (see above)

3. **Add to BinaryOperator enum** in `src/ast.rs`:
```rust
pub enum BinaryOperator {
    Add, Sub, Mul, Div,  // Add Mul here
}
```

4. **Add parsing logic** in `parse_expression_with_precedence()`:
```rust
Token::Multiply => {
    self.advance();
    let right = self.parse_expression_with_precedence(Precedence::Multiplication)?;
    left = Expression::BinaryOp(Box::new(left), BinaryOperator::Mul, Box::new(right));
}
```

## 3. The AST (`src/ast.rs`)

The AST represents the parsed structure in memory.

### Key Structures

```rust
pub struct HelixAst {
    declarations: Vec<Declaration>,
}

pub struct SectionDecl {
    pub name: String,
    pub properties: HashMap<String, Expression>,
}
```

### Expression Evaluation

Expressions are evaluated by the `OperatorParser` in `src/ops.rs` or the `HelixInterpreter` in `src/interpreter.rs`.

## 4. The Evaluator (`src/ops.rs`)

The `OperatorParser` evaluates expressions and handles @ operators.

### @ Operator System

@ operators are special functions like `@env`, `@date`, `@query`, `@transform`. Helix supports flexible syntax including cross-file references, shortcuts, and data transformations:

```rust
// Enhanced @env operator supports multiple syntaxes
let env_re = Regex::new(r#"^@env\[["']([^"']*)["']\]|@env\(["']([^"']*)["'](?:,\s*(.+))?\)$"#).unwrap();

// @ shortcut syntax for section access
let section_access_re = Regex::new(r#"^@([a-zA-Z_][a-zA-Z0-9_]*)\[["']([^"']*)["']\]$"#).unwrap();
let nested_access_re = Regex::new(r#"^@([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\[["']([^"']*)["']\]$"#).unwrap();

// @transform for ML data format conversion
let transform_re = Regex::new(r#"^@transform\(["']([^"']*)["'],\s*(.+)\)$"#).unwrap();

// Cross-file references
let cross_get_re = Regex::new(r#"^@([a-zA-Z0-9_-]+)\.hlx\.get\(["'](.*)["']\)$"#).unwrap();
```

### Adding New @ Operators

To add a new @ operator (like `@random`):

1. **Add regex pattern** in `parse_value()`:
```rust
let random_re = Regex::new(r"^@random\((\d+), (\d+)\)$").unwrap();
if let Some(captures) = random_re.captures(value) {
    let min: i32 = captures.get(1).unwrap().as_str().parse().unwrap();
    let max: i32 = captures.get(2).unwrap().as_str().parse().unwrap();
    let random_value = (rand::random::<i32>() % (max - min + 1)) + min;
    return Ok(Value::Number(random_value as f64));
}
```

## 5. Enhancement Workflow

### Adding a New Feature: Step-by-Step

**Example: Adding Task Keyword Support**

Task declarations were recently added to Helix. Here's how it was implemented:

1. **Add Task Keyword** (lexer):
```rust
pub enum Keyword {
    // ... existing keywords
    Task,  // Added for task declarations
}
"task" => Some(Keyword::Task),  // Added mapping
```

2. **Add Task Declaration** (parser):
```rust
match keyword {
    Keyword::Task => {
        self.advance();
        let (name, block_kind) = self.parse_generic_variations("task".to_string())?;
        match block_kind {
            BlockKind::Brace | BlockKind::Angle => {
                let properties = self.parse_properties()?;
                // Handle closing delimiter...
                Ok(Declaration::Task(TaskDecl { name, properties }))
            }
            // ... other block types
        }
    }
    // ... existing keywords
}
```

3. **Add TaskDecl** (AST):
```rust
#[derive(Debug, Clone)]
pub struct TaskDecl {
    pub name: String,
    pub properties: HashMap<String, Expression>,
}
pub enum Declaration {
    // ... existing declarations
    Task(TaskDecl),  // Added task support
    Section(SectionDecl),
}
```

4. **Add Type Conversion** (types.rs):
```rust
crate::ast::Declaration::Task(t) => {
    let task_data: HashMap<String, Value> = t
        .properties
        .iter()
        .map(|(k, v)| (k.clone(), v.to_value()))
        .collect();
    config.sections.insert(format!("task.{}", t.name), task_data);
}
```

**Example: Adding Binary Operators**

The `+` operator for string concatenation was recently added:

1. **Add Plus Token** (lexer):
```rust
Some('+') => {
    self.advance();
    Token::Plus
}
```

2. **Add Precedence** (parser):
```rust
fn get_token_precedence(&self, token: &Token) -> Precedence {
    match token {
        Token::Plus => Precedence::Addition,
        // ... other precedences
    }
}
```

3. **Add Parsing Logic** (parser):
```rust
Token::Plus => {
    self.advance();
    let right = self.parse_expression_with_precedence(Precedence::Addition)?;
    left = Expression::BinaryOp(Box::new(left), BinaryOperator::Add, Box::new(right));
}
```

### Syntax Patterns

Helix supports rich syntax patterns with multiple block delimiters and advanced expressions:

```hlx
# Keyword declarations with named parameters
project "myapp" {
    name = "My Application"
    version = "1.0.0"
}

# Task declarations (recently added)
task "data_cleanup" <
    schedule = !CLEANUP_SCHEDULE!
    enabled = !CLEANUP_ENABLED!
    batch_size = !BATCH_SIZE!
>

# Generic sections with subnames (recently added)
app core {}
service "api" {}
module auth : ;

# All block delimiters work: {}, <>, [], : ;
config_braces { type = "braces" }
config_angles < type = "angles" >
config_brackets [ type = "brackets" ]
config_colon : type = "colon" ;

# Generic sections with ~
~deployment <
    region = @env["AWS_REGION"]
    account = @env['AWS_ACCOUNT']
    environment = @env['ENVIRONMENT']
>

# @ shortcuts for section access (recently added)
style_test :
    name = "Style Test"
    version = "1.0"
;

project "demo" {
    # Direct section access
    test_name = @style_test['name']
    test_version = @style_test['version']
}

# @transform for ML data format conversion (recently added)
data_pipeline :
    raw_data = [
        {"prompt": "Hello", "completion": "Hi there!"},
        {"chosen": "Good response", "rejected": "Bad response"}
    ]

    # Convert between ML training formats
    conversational = @transform("conversational", @data_pipeline.raw_data)
    preference = @transform("preference", @data_pipeline.raw_data)
    chatml = @transform("chatml", @data_pipeline.raw_data)

# Variable markers and expressions
~api <
    base_url = @env['API_BASE_URL'] + "/api/v1"
    timeout = !API_TIMEOUT!
    debug = $DEBUG_MODE
    allowed_origins = [
        @env['ORIGIN_1'],
        !CUSTOM_ORIGIN!
    ]
    headers = {
        "X-API-Key" = !API_KEY!,
        "Authorization" = "Bearer " + @env['AUTH_TOKEN']
    }
>

# Multiple block types
workflow "main" {
    trigger = "manual"
    step "process" {
        agent = "worker"
        task = "process_data"
    }
}
```

## 6. Testing & Debugging

### Validation Command

```bash
./target/debug/hlx validate your_file.hlx
```

### Schema Generation (8+ Languages)

Generate SDKs in multiple programming languages:

```bash
# Generate Python SDK
./target/debug/hlx schema your_file.hlx --lang python

# Generate JavaScript SDK
./target/debug/hlx schema your_file.hlx --lang javascript

# Supported languages: rust, python, javascript, csharp, java, go, ruby, php
./target/debug/hlx schema your_file.hlx --lang csharp --output MyConfig.cs
```

### Common Issues

1. **"Expected block delimiter after 'X'"** → Missing `{`, `<`, `[`, or `:` after declaration name
2. **"Expected ']' after argument of @env"** → @env syntax error, use `@env["VAR"]` or `@env("VAR")`
3. **"Expected '=' after property key '/'"** → Used `//` comments instead of `#` (Helix only supports `#` comments)
4. **"Unexpected token"** → New syntax not handled by parser (add to lexer/parser)
5. **"Unknown operator"** → @ operator not implemented in `src/ops.rs`
6. **"Expected '=' after property key"** → Binary operators like `+` not tokenized (add to lexer)

### Debug Tips

- Use minimal test files to isolate issues
- Check tokenization with debug prints
- Verify AST structure matches expectations
- Test evaluation separately from parsing

## 7. Best Practices

1. **Start Small**: Test new features with minimal examples
2. **Follow Patterns**: Use existing syntax patterns when possible  
3. **Handle Errors**: Provide clear error messages
4. **Document**: Update this guide when adding features
5. **Test Thoroughly**: Cover parsing, evaluation, and edge cases

## 8. Advanced Topics

### Current System Capabilities

Helix currently supports:

- **8+ Programming Languages** for schema generation (Rust, Python, JavaScript, C#, Java, Go, Ruby, PHP)
- **Multiple Block Syntaxes**: `{}`, `<>`, `[]`, `:`
- **Advanced Expressions**: Binary operators (`+`), @ operators, variables (`$VAR`, `!MARKER!`)
- **Complex Data Types**: Objects, arrays, strings with concatenation
- **Environment Integration**: `@env` with multiple syntaxes and defaults
- **Cross-File References**: `@file.hlx.get()`, `@file.hlx.set()`
- **Data Transformation**: `@transform("template", data)` for ML training data conversion

### Recent Enhancements

1. **Task Keyword**: Added `task "name" <>` declarations for workflow automation
2. **Generic Subnames**: Support for `section subname {}` and `section "subname" {}` syntax
3. **Colon Syntax**: Fixed `: ;` blocks to support properties (not just empty sections)
4. **Binary Operators**: Implemented `+` for string concatenation and arithmetic
5. **@ Shortcuts**: Added `@section['key']` and `@section.property['key']` for cross-section access
6. **@ Transform Operator**: Added `@transform("template", data)` for ML data format conversion
7. **Enhanced @env**: Support for both `@env["VAR"]` and `@env("VAR", "default")` syntaxes
8. **Schema Generation**: Automatic SDK generation in 8 programming languages
9. **Angle Bracket Syntax**: Improved `<>` block parsing and validation

### Custom Operators

Operators can be implemented in `src/operators/` directory with async support for complex operations like database queries, API calls, and external integrations.

### Cross-File References

```hlx
# Reference other .hlx files
@other.hlx.get('config.database.host')
@other.hlx.set('config.cache.enabled', true)
```

### Runtime Context

The parser maintains runtime context for variable resolution and operator execution, supporting both synchronous and asynchronous evaluation.

## 9. Contributing

### Code Quality Standards

- All new features must include comprehensive tests
- Update this documentation when adding new syntax or features
- Follow existing code patterns and error handling conventions
- Ensure backward compatibility with existing `.hlx` files

### Testing Strategy

1. **Unit Tests**: Test individual components (lexer, parser, evaluator)
2. **Integration Tests**: Test complete `.hlx` file processing
3. **Schema Generation**: Verify generated SDKs compile and work correctly
4. **Regression Tests**: Ensure existing functionality remains intact

---

This guide reflects the current state of the Helix system as of the latest enhancements. The system continues to evolve with new features, improved syntax, and expanded language support. Remember: small, incremental changes with thorough testing lead to robust enhancements.
