use rustpython_parser::ast::{self, ArgWithDefault, Constant, Expr, Ranged, Stmt};

use ruff_diagnostics::{AlwaysAutofixableViolation, Diagnostic, Fix, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::cast;
use ruff_python_ast::helpers::ReturnStatementVisitor;
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::statement_visitor::StatementVisitor;
use ruff_python_ast::typing::parse_type_annotation;
use ruff_python_semantic::analyze::visibility;
use ruff_python_semantic::{Definition, Member, MemberKind};
use ruff_python_stdlib::typing::simple_magic_return_type;

use crate::checkers::ast::Checker;
use crate::registry::{AsRule, Rule};
use crate::rules::ruff::typing::type_hint_resolves_to_any;

use super::super::fixes;
use super::super::helpers::match_function_def;

/// ## What it does
/// Checks that function arguments have type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the types of function arguments. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any provided arguments match expectation.
///
/// ## Example
/// ```python
/// def foo(x):
///     ...
/// ```
///
/// Use instead:
/// ```python
/// def foo(x: int):
///     ...
/// ```
#[violation]
pub struct MissingTypeFunctionArgument {
    name: String,
}

impl Violation for MissingTypeFunctionArgument {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingTypeFunctionArgument { name } = self;
        format!("Missing type annotation for function argument `{name}`")
    }
}

/// ## What it does
/// Checks that function `*args` arguments have type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the types of function arguments. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any provided arguments match expectation.
///
/// ## Example
/// ```python
/// def foo(*args):
///     ...
/// ```
///
/// Use instead:
/// ```python
/// def foo(*args: int):
///     ...
/// ```
#[violation]
pub struct MissingTypeArgs {
    name: String,
}

impl Violation for MissingTypeArgs {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingTypeArgs { name } = self;
        format!("Missing type annotation for `*{name}`")
    }
}

/// ## What it does
/// Checks that function `**kwargs` arguments have type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the types of function arguments. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any provided arguments match expectation.
///
/// ## Example
/// ```python
/// def foo(**kwargs):
///     ...
/// ```
///
/// Use instead:
/// ```python
/// def foo(**kwargs: int):
///     ...
/// ```
#[violation]
pub struct MissingTypeKwargs {
    name: String,
}

impl Violation for MissingTypeKwargs {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingTypeKwargs { name } = self;
        format!("Missing type annotation for `**{name}`")
    }
}

/// ## What it does
/// Checks that instance method `self` arguments have type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the types of function arguments. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any provided arguments match expectation.
///
/// Note that many type checkers will infer the type of `self` automatically, so this
/// annotation is not strictly necessary.
///
/// ## Example
/// ```python
/// class Foo:
///     def bar(self):
///         ...
/// ```
///
/// Use instead:
/// ```python
/// class Foo:
///     def bar(self: "Foo"):
///         ...
/// ```
#[violation]
pub struct MissingTypeSelf {
    name: String,
}

impl Violation for MissingTypeSelf {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingTypeSelf { name } = self;
        format!("Missing type annotation for `{name}` in method")
    }
}

/// ## What it does
/// Checks that class method `cls` arguments have type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the types of function arguments. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any provided arguments match expectation.
///
/// Note that many type checkers will infer the type of `cls` automatically, so this
/// annotation is not strictly necessary.
///
/// ## Example
/// ```python
/// class Foo:
///     @classmethod
///     def bar(cls):
///         ...
/// ```
///
/// Use instead:
/// ```python
/// class Foo:
///     @classmethod
///     def bar(cls: Type["Foo"]):
///         ...
/// ```
#[violation]
pub struct MissingTypeCls {
    name: String,
}

impl Violation for MissingTypeCls {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingTypeCls { name } = self;
        format!("Missing type annotation for `{name}` in classmethod")
    }
}

/// ## What it does
/// Checks that public functions and methods have return type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the return types of functions. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any returned values, and the types expected by callers, match expectation.
///
/// ## Example
/// ```python
/// def add(a, b):
///     return a + b
/// ```
///
/// Use instead:
/// ```python
/// def add(a: int, b: int) -> int:
///     return a + b
/// ```
#[violation]
pub struct MissingReturnTypeUndocumentedPublicFunction {
    name: String,
}

impl Violation for MissingReturnTypeUndocumentedPublicFunction {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingReturnTypeUndocumentedPublicFunction { name } = self;
        format!("Missing return type annotation for public function `{name}`")
    }
}

/// ## What it does
/// Checks that private functions and methods have return type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the return types of functions. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any returned values, and the types expected by callers, match expectation.
///
/// ## Example
/// ```python
/// def _add(a, b):
///     return a + b
/// ```
///
/// Use instead:
/// ```python
/// def _add(a: int, b: int) -> int:
///     return a + b
/// ```
#[violation]
pub struct MissingReturnTypePrivateFunction {
    name: String,
}

impl Violation for MissingReturnTypePrivateFunction {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingReturnTypePrivateFunction { name } = self;
        format!("Missing return type annotation for private function `{name}`")
    }
}

/// ## What it does
/// Checks that "special" methods, like `__init__`, `__new__`, and `__call__`, have
/// return type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the return types of functions. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any returned values, and the types expected by callers, match expectation.
///
/// Note that type checkers often allow you to omit the return type annotation for
/// `__init__` methods, as long as at least one argument has a type annotation. To
/// opt-in to this behavior, use the `mypy-init-return` setting in your `pyproject.toml`
/// or `ruff.toml` file:
///
/// ```toml
/// [tool.ruff.flake8-annotations]
/// mypy-init-return = true
/// ```
///
/// ## Example
/// ```python
/// class Foo:
///     def __init__(self, x: int):
///         self.x = x
/// ```
///
/// Use instead:
/// ```python
/// class Foo:
///     def __init__(self, x: int) -> None:
///         self.x = x
/// ```
#[violation]
pub struct MissingReturnTypeSpecialMethod {
    name: String,
}

impl AlwaysAutofixableViolation for MissingReturnTypeSpecialMethod {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingReturnTypeSpecialMethod { name } = self;
        format!("Missing return type annotation for special method `{name}`")
    }

    fn autofix_title(&self) -> String {
        "Add `None` return type".to_string()
    }
}

/// ## What it does
/// Checks that static methods have return type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the return types of functions. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any returned values, and the types expected by callers, match expectation.
///
/// ## Example
/// ```python
/// class Foo:
///     @staticmethod
///     def bar():
///         return 1
/// ```
///
/// Use instead:
/// ```python
/// class Foo:
///     @staticmethod
///     def bar() -> int:
///         return 1
/// ```
#[violation]
pub struct MissingReturnTypeStaticMethod {
    name: String,
}

impl Violation for MissingReturnTypeStaticMethod {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingReturnTypeStaticMethod { name } = self;
        format!("Missing return type annotation for staticmethod `{name}`")
    }
}

/// ## What it does
/// Checks that class methods have return type annotations.
///
/// ## Why is this bad?
/// Type annotations are a good way to document the return types of functions. They also
/// help catch bugs, when used alongside a type checker, by ensuring that the types of
/// any returned values, and the types expected by callers, match expectation.
///
/// ## Example
/// ```python
/// class Foo:
///     @classmethod
///     def bar(cls):
///         return 1
/// ```
///
/// Use instead:
/// ```python
/// class Foo:
///     @classmethod
///     def bar(cls) -> int:
///         return 1
/// ```
#[violation]
pub struct MissingReturnTypeClassMethod {
    name: String,
}

impl Violation for MissingReturnTypeClassMethod {
    #[derive_message_formats]
    fn message(&self) -> String {
        let MissingReturnTypeClassMethod { name } = self;
        format!("Missing return type annotation for classmethod `{name}`")
    }
}

/// ## What it does
/// Checks that an expression is annotated with a more specific type than
/// `Any`.
///
/// ## Why is this bad?
/// `Any` is a special type indicating an unconstrained type. When an
/// expression is annotated with type `Any`, type checkers will allow all
/// operations on it.
///
/// It's better to be explicit about the type of an expression, and to use
/// `Any` as an "escape hatch" only when it is really needed.
///
/// ## Example
/// ```python
/// def foo(x: Any):
///     ...
/// ```
///
/// Use instead:
/// ```python
/// def foo(x: int):
///     ...
/// ```
///
/// ## References
/// - [PEP 484](https://www.python.org/dev/peps/pep-0484/#the-any-type)
/// - [`typing.Any`](https://docs.python.org/3/library/typing.html#typing.Any)
/// - [Mypy: The Any type](https://mypy.readthedocs.io/en/stable/kinds_of_types.html#the-any-type)
#[violation]
pub struct AnyType {
    name: String,
}

impl Violation for AnyType {
    #[derive_message_formats]
    fn message(&self) -> String {
        let AnyType { name } = self;
        format!("Dynamically typed expressions (typing.Any) are disallowed in `{name}`")
    }
}

fn is_none_returning(body: &[Stmt]) -> bool {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(body);
    for stmt in visitor.returns {
        if let Some(value) = stmt.value.as_deref() {
            if !matches!(
                value,
                Expr::Constant(constant) if constant.value.is_none()
            ) {
                return false;
            }
        }
    }
    true
}

/// ANN401
fn check_dynamically_typed<F>(
    checker: &Checker,
    annotation: &Expr,
    func: F,
    diagnostics: &mut Vec<Diagnostic>,
) where
    F: FnOnce() -> String,
{
    if let Expr::Constant(ast::ExprConstant {
        range,
        value: Constant::Str(string),
        ..
    }) = annotation
    {
        // Quoted annotations
        if let Ok((parsed_annotation, _)) = parse_type_annotation(string, *range, checker.locator) {
            if type_hint_resolves_to_any(
                &parsed_annotation,
                checker.semantic(),
                checker.locator,
                checker.settings.target_version.minor(),
            ) {
                diagnostics.push(Diagnostic::new(
                    AnyType { name: func() },
                    annotation.range(),
                ));
            }
        }
    } else {
        if type_hint_resolves_to_any(
            annotation,
            checker.semantic(),
            checker.locator,
            checker.settings.target_version.minor(),
        ) {
            diagnostics.push(Diagnostic::new(
                AnyType { name: func() },
                annotation.range(),
            ));
        }
    }
}

/// Generate flake8-annotation checks for a given `Definition`.
pub(crate) fn definition(
    checker: &Checker,
    definition: &Definition,
    visibility: visibility::Visibility,
) -> Vec<Diagnostic> {
    // TODO(charlie): Consider using the AST directly here rather than `Definition`.
    // We could adhere more closely to `flake8-annotations` by defining public
    // vs. secret vs. protected.
    let Definition::Member(Member { kind, stmt, .. }) = definition else {
        return vec![];
    };

    let is_method = match kind {
        MemberKind::Method => true,
        MemberKind::Function | MemberKind::NestedFunction => false,
        _ => return vec![],
    };

    let (name, arguments, returns, body, decorator_list) = match_function_def(stmt);
    // Keep track of whether we've seen any typed arguments or return values.
    let mut has_any_typed_arg = false; // Any argument has been typed?
    let mut has_typed_return = false; // Return value has been typed?
    let mut has_typed_self_or_cls = false; // Has a typed `self` or `cls` argument?

    // Temporary storage for diagnostics; we emit them at the end
    // unless configured to suppress ANN* for declarations that are fully untyped.
    let mut diagnostics = Vec::new();

    let is_overridden = visibility::is_override(decorator_list, checker.semantic());

    // ANN001, ANN401
    for ArgWithDefault {
        def,
        default: _,
        range: _,
    } in arguments
        .posonlyargs
        .iter()
        .chain(&arguments.args)
        .chain(&arguments.kwonlyargs)
        .skip(
            // If this is a non-static method, skip `cls` or `self`.
            usize::from(
                is_method
                    && !visibility::is_staticmethod(cast::decorator_list(stmt), checker.semantic()),
            ),
        )
    {
        // ANN401 for dynamically typed arguments
        if let Some(annotation) = &def.annotation {
            has_any_typed_arg = true;
            if checker.enabled(Rule::AnyType) && !is_overridden {
                check_dynamically_typed(
                    checker,
                    annotation,
                    || def.arg.to_string(),
                    &mut diagnostics,
                );
            }
        } else {
            if !(checker.settings.flake8_annotations.suppress_dummy_args
                && checker.settings.dummy_variable_rgx.is_match(&def.arg))
            {
                if checker.enabled(Rule::MissingTypeFunctionArgument) {
                    diagnostics.push(Diagnostic::new(
                        MissingTypeFunctionArgument {
                            name: def.arg.to_string(),
                        },
                        def.range(),
                    ));
                }
            }
        }
    }

    // ANN002, ANN401
    if let Some(arg) = &arguments.vararg {
        if let Some(expr) = &arg.annotation {
            has_any_typed_arg = true;
            if !checker.settings.flake8_annotations.allow_star_arg_any {
                if checker.enabled(Rule::AnyType) && !is_overridden {
                    let name = &arg.arg;
                    check_dynamically_typed(checker, expr, || format!("*{name}"), &mut diagnostics);
                }
            }
        } else {
            if !(checker.settings.flake8_annotations.suppress_dummy_args
                && checker.settings.dummy_variable_rgx.is_match(&arg.arg))
            {
                if checker.enabled(Rule::MissingTypeArgs) {
                    diagnostics.push(Diagnostic::new(
                        MissingTypeArgs {
                            name: arg.arg.to_string(),
                        },
                        arg.range(),
                    ));
                }
            }
        }
    }

    // ANN003, ANN401
    if let Some(arg) = &arguments.kwarg {
        if let Some(expr) = &arg.annotation {
            has_any_typed_arg = true;
            if !checker.settings.flake8_annotations.allow_star_arg_any {
                if checker.enabled(Rule::AnyType) && !is_overridden {
                    let name = &arg.arg;
                    check_dynamically_typed(
                        checker,
                        expr,
                        || format!("**{name}"),
                        &mut diagnostics,
                    );
                }
            }
        } else {
            if !(checker.settings.flake8_annotations.suppress_dummy_args
                && checker.settings.dummy_variable_rgx.is_match(&arg.arg))
            {
                if checker.enabled(Rule::MissingTypeKwargs) {
                    diagnostics.push(Diagnostic::new(
                        MissingTypeKwargs {
                            name: arg.arg.to_string(),
                        },
                        arg.range(),
                    ));
                }
            }
        }
    }

    // ANN101, ANN102
    if is_method && !visibility::is_staticmethod(cast::decorator_list(stmt), checker.semantic()) {
        if let Some(ArgWithDefault {
            def,
            default: _,
            range: _,
        }) = arguments
            .posonlyargs
            .first()
            .or_else(|| arguments.args.first())
        {
            if def.annotation.is_none() {
                if visibility::is_classmethod(cast::decorator_list(stmt), checker.semantic()) {
                    if checker.enabled(Rule::MissingTypeCls) {
                        diagnostics.push(Diagnostic::new(
                            MissingTypeCls {
                                name: def.arg.to_string(),
                            },
                            def.range(),
                        ));
                    }
                } else {
                    if checker.enabled(Rule::MissingTypeSelf) {
                        diagnostics.push(Diagnostic::new(
                            MissingTypeSelf {
                                name: def.arg.to_string(),
                            },
                            def.range(),
                        ));
                    }
                }
            } else {
                has_typed_self_or_cls = true;
            }
        }
    }

    // ANN201, ANN202, ANN401
    if let Some(expr) = &returns {
        has_typed_return = true;
        if checker.enabled(Rule::AnyType) && !is_overridden {
            check_dynamically_typed(checker, expr, || name.to_string(), &mut diagnostics);
        }
    } else if !(
        // Allow omission of return annotation if the function only returns `None`
        // (explicitly or implicitly).
        checker.settings.flake8_annotations.suppress_none_returning && is_none_returning(body)
    ) {
        if is_method && visibility::is_classmethod(cast::decorator_list(stmt), checker.semantic()) {
            if checker.enabled(Rule::MissingReturnTypeClassMethod) {
                diagnostics.push(Diagnostic::new(
                    MissingReturnTypeClassMethod {
                        name: name.to_string(),
                    },
                    stmt.identifier(),
                ));
            }
        } else if is_method
            && visibility::is_staticmethod(cast::decorator_list(stmt), checker.semantic())
        {
            if checker.enabled(Rule::MissingReturnTypeStaticMethod) {
                diagnostics.push(Diagnostic::new(
                    MissingReturnTypeStaticMethod {
                        name: name.to_string(),
                    },
                    stmt.identifier(),
                ));
            }
        } else if is_method && visibility::is_init(name) {
            // Allow omission of return annotation in `__init__` functions, as long as at
            // least one argument is typed.
            if checker.enabled(Rule::MissingReturnTypeSpecialMethod) {
                if !(checker.settings.flake8_annotations.mypy_init_return && has_any_typed_arg) {
                    let mut diagnostic = Diagnostic::new(
                        MissingReturnTypeSpecialMethod {
                            name: name.to_string(),
                        },
                        stmt.identifier(),
                    );
                    if checker.patch(diagnostic.kind.rule()) {
                        diagnostic.try_set_fix(|| {
                            fixes::add_return_annotation(checker.locator, stmt, "None")
                                .map(Fix::suggested)
                        });
                    }
                    diagnostics.push(diagnostic);
                }
            }
        } else if is_method && visibility::is_magic(name) {
            if checker.enabled(Rule::MissingReturnTypeSpecialMethod) {
                let mut diagnostic = Diagnostic::new(
                    MissingReturnTypeSpecialMethod {
                        name: name.to_string(),
                    },
                    stmt.identifier(),
                );
                if checker.patch(diagnostic.kind.rule()) {
                    if let Some(return_type) = simple_magic_return_type(name) {
                        diagnostic.try_set_fix(|| {
                            fixes::add_return_annotation(checker.locator, stmt, return_type)
                                .map(Fix::suggested)
                        });
                    }
                }
                diagnostics.push(diagnostic);
            }
        } else {
            match visibility {
                visibility::Visibility::Public => {
                    if checker.enabled(Rule::MissingReturnTypeUndocumentedPublicFunction) {
                        diagnostics.push(Diagnostic::new(
                            MissingReturnTypeUndocumentedPublicFunction {
                                name: name.to_string(),
                            },
                            stmt.identifier(),
                        ));
                    }
                }
                visibility::Visibility::Private => {
                    if checker.enabled(Rule::MissingReturnTypePrivateFunction) {
                        diagnostics.push(Diagnostic::new(
                            MissingReturnTypePrivateFunction {
                                name: name.to_string(),
                            },
                            stmt.identifier(),
                        ));
                    }
                }
            }
        }
    }
    // If settings say so, don't report any of the
    // diagnostics gathered here if there were no type annotations at all.
    if checker.settings.flake8_annotations.ignore_fully_untyped
        && !(has_any_typed_arg || has_typed_self_or_cls || has_typed_return)
    {
        vec![]
    } else {
        diagnostics
    }
}
