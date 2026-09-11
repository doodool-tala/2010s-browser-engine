//! Minimal WebIDL subset parser. Normative reference: ADR-0007.
//!
//! v1 subset: `interface Name : Parent` blocks containing `attribute`
//! and `readonly attribute` declarations and methods, over the types
//! unsigned long, long, boolean, double, and DOMString. Comments
//! (`//` and `/* */`) are skipped.

use nbe_core::error::ModuleError;

/// An IDL type understood by the v1 subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdlType {
    /// `unsigned long`
    UnsignedLong,
    /// `long`
    Long,
    /// `boolean`
    Boolean,
    /// `double`
    Double,
    /// `DOMString`
    DomString,
}

impl IdlType {
    /// The Rust type for values and returns of this IDL type.
    #[must_use]
    pub fn rust_value_type(self) -> &'static str {
        match self {
            IdlType::UnsignedLong => "u32",
            IdlType::Long => "i32",
            IdlType::Boolean => "bool",
            IdlType::Double => "f64",
            IdlType::DomString => "String",
        }
    }

    /// The Rust type for parameters of this IDL type.
    #[must_use]
    pub fn rust_param_type(self) -> &'static str {
        match self {
            IdlType::DomString => "&str",
            other => other.rust_value_type(),
        }
    }
}

/// One interface member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    /// An attribute declaration.
    Attribute {
        /// Attribute type.
        ty: IdlType,
        /// Attribute name, lowerCamelCase as written in IDL.
        name: String,
        /// True when declared `readonly`.
        readonly: bool,
    },
    /// A method declaration.
    Method {
        /// Return type; `None` means `void`.
        ret: Option<IdlType>,
        /// Method name, lowerCamelCase as written in IDL.
        name: String,
        /// Arguments in declaration order.
        args: Vec<Argument>,
    },
}

/// One method argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    /// Argument type.
    pub ty: IdlType,
    /// Argument name.
    pub name: String,
}

/// One parsed interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// Interface name.
    pub name: String,
    /// Parent interface name, when inherited.
    pub parent: Option<String>,
    /// Members in declaration order.
    pub members: Vec<Member>,
}

/// Parse IDL source text into interfaces, in declaration order.
///
/// # Errors
/// Module error ("bindings-idl") on any lexical or syntax violation.
pub fn parse_idl(source: &str) -> Result<Vec<Interface>, ModuleError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_interfaces()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    Symbol(char),
}

/// Split IDL source into tokens, skipping whitespace and comments.
fn tokenize(source: &str) -> Result<Vec<Token>, ModuleError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '/' => {
                chars.next();
                match chars.next() {
                    Some('/') => {
                        for c in chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    }
                    Some('*') => {
                        let mut closed = false;
                        let mut previous = '\0';
                        for c in chars.by_ref() {
                            if previous == '*' && c == '/' {
                                closed = true;
                                break;
                            }
                            previous = c;
                        }
                        if !closed {
                            return Err(ModuleError::new(
                                "bindings-idl",
                                "unterminated block comment",
                            ));
                        }
                    }
                    other => {
                        return Err(ModuleError::new(
                            "bindings-idl",
                            format!("bad character after '/': {other:?}"),
                        ));
                    }
                }
            }
            '{' | '}' | '(' | ')' | ';' | ',' | ':' => {
                chars.next();
                tokens.push(Token::Symbol(ch));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(name));
            }
            other => {
                return Err(ModuleError::new(
                    "bindings-idl",
                    format!("unexpected character: {other:?}"),
                ));
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn err<T>(&self, message: &str) -> Result<T, ModuleError> {
        Err(ModuleError::new(
            "bindings-idl",
            format!("at token {message}"),
        ))
    }

    fn expect_symbol(&mut self, symbol: char) -> Result<(), ModuleError> {
        match self.next() {
            Some(Token::Symbol(s)) if s == symbol => Ok(()),
            other => self.err(&format!("expected '{symbol}', got {other:?}")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ModuleError> {
        match self.next() {
            Some(Token::Ident(name)) => Ok(name),
            other => self.err(&format!("expected identifier, got {other:?}")),
        }
    }

    fn parse_interfaces(&mut self) -> Result<Vec<Interface>, ModuleError> {
        let mut interfaces = Vec::new();
        while self.peek().is_some() {
            let keyword = self.expect_ident()?;
            if keyword != "interface" {
                return self.err("expected 'interface'");
            }
            let name = self.expect_ident()?;
            let parent = if matches!(self.peek(), Some(Token::Symbol(':'))) {
                self.expect_symbol(':')?;
                Some(self.expect_ident()?)
            } else {
                None
            };
            self.expect_symbol('{')?;
            let members = self.parse_members()?;
            self.expect_symbol(';')?;
            interfaces.push(Interface {
                name,
                parent,
                members,
            });
        }
        Ok(interfaces)
    }

    fn parse_members(&mut self) -> Result<Vec<Member>, ModuleError> {
        let mut members = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::Symbol('}'))) {
                self.expect_symbol('}')?;
                return Ok(members);
            }
            members.push(self.parse_member()?);
        }
    }

    fn parse_member(&mut self) -> Result<Member, ModuleError> {
        let first = self.expect_ident()?;
        let readonly = first == "readonly";
        let keyword = if readonly {
            self.expect_ident()?
        } else {
            first
        };
        if keyword == "attribute" {
            let ty = self.parse_type()?;
            let name = self.expect_ident()?;
            self.expect_symbol(';')?;
            return Ok(Member::Attribute { ty, name, readonly });
        }
        let ret = if keyword == "void" {
            None
        } else {
            Some(self.parse_type_from(keyword)?)
        };
        let name = self.expect_ident()?;
        self.expect_symbol('(')?;
        let mut args = Vec::new();
        if !matches!(self.peek(), Some(Token::Symbol(')'))) {
            loop {
                let ty = self.parse_type()?;
                let arg_name = self.expect_ident()?;
                args.push(Argument { ty, name: arg_name });
                if matches!(self.peek(), Some(Token::Symbol(','))) {
                    self.expect_symbol(',')?;
                } else {
                    break;
                }
            }
        }
        self.expect_symbol(')')?;
        self.expect_symbol(';')?;
        Ok(Member::Method { ret, name, args })
    }

    fn parse_type(&mut self) -> Result<IdlType, ModuleError> {
        let first = self.expect_ident()?;
        self.parse_type_from(first)
    }

    fn parse_type_from(&mut self, first: String) -> Result<IdlType, ModuleError> {
        match first.as_str() {
            "unsigned" => {
                let second = self.expect_ident()?;
                if second == "long" {
                    Ok(IdlType::UnsignedLong)
                } else {
                    self.err("expected 'long' after 'unsigned'")
                }
            }
            "long" => Ok(IdlType::Long),
            "boolean" => Ok(IdlType::Boolean),
            "double" => Ok(IdlType::Double),
            "DOMString" => Ok(IdlType::DomString),
            _ => self.err("unsupported IDL type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
// line comment
/* block comment */
interface Node {
  readonly attribute unsigned long nodeType;
  attribute DOMString nodeName;
};

interface Element : Node {
  attribute DOMString tagName;
  boolean check(DOMString name, long delta);
};
"#;

    #[test]
    fn parses_interfaces_attributes_and_methods() {
        let interfaces = parse_idl(SAMPLE).unwrap();
        assert_eq!(interfaces.len(), 2);
        assert_eq!(interfaces[0].name, "Node");
        assert_eq!(interfaces[0].parent, None);
        assert_eq!(
            interfaces[0].members,
            vec![
                Member::Attribute {
                    ty: IdlType::UnsignedLong,
                    name: "nodeType".into(),
                    readonly: true,
                },
                Member::Attribute {
                    ty: IdlType::DomString,
                    name: "nodeName".into(),
                    readonly: false,
                },
            ]
        );
        assert_eq!(interfaces[1].name, "Element");
        assert_eq!(interfaces[1].parent.as_deref(), Some("Node"));
        match &interfaces[1].members[1] {
            Member::Method { ret, name, args } => {
                assert_eq!(*ret, Some(IdlType::Boolean));
                assert_eq!(name, "check");
                assert_eq!(args.len(), 2);
                assert_eq!(
                    args[0],
                    Argument {
                        ty: IdlType::DomString,
                        name: "name".into(),
                    }
                );
                assert_eq!(
                    args[1],
                    Argument {
                        ty: IdlType::Long,
                        name: "delta".into(),
                    }
                );
            }
            other => panic!("expected method, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_idl() {
        assert!(parse_idl("interface {").is_err());
        assert!(parse_idl("interface Node { attribute; }").is_err());
        assert!(parse_idl("interface Node { weirdType attr; }").is_err());
    }
}
