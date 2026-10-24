use std::fmt;

use serde::Serialize;
use topcoat_core::{context::Cx, fnv1a::Fnv1a};
use topcoat_view::{AttributeValueViewParts, PartsWriter};

/// JavaScript for a runtime expression, as data the browser runtime evaluates.
///
/// A `Js` is either a [`JsTemplate`] applied to holes, which are themselves
/// `Js`, or a captured Rust value serialized as JSON. Its [`Display`](fmt::Display) form is
/// the JSON the browser receives: a template renders as `["<id>", hole, ...]`
/// and a captured value as `{"v": <json>}`. The browser looks templates up by
/// id in the module served from [`JsModule`](crate::JsModule), so pages never
/// carry executable source.
///
/// Embedding a `Js` in a view escapes it for the surrounding HTML context.
#[derive(Debug, Clone)]
pub struct Js(JsNode);

#[derive(Debug, Clone)]
enum JsNode {
    Template {
        template: &'static JsTemplate,
        holes: Vec<Js>,
    },
    Surrogate(String),
}

impl Js {
    /// Applies `template` to `holes`.
    ///
    /// The browser passes the holes to the template as functions, in order.
    /// Each call evaluates its hole again. The browser only finds templates
    /// registered with [`js_template!`](crate::js_template).
    #[doc(hidden)]
    #[must_use]
    pub fn template(template: &'static JsTemplate, holes: Vec<Self>) -> Self {
        Self(JsNode::Template { template, holes })
    }

    /// A captured value, serialized now so the Rust expression can consume it
    /// afterwards. The browser hydrates it each time it is evaluated.
    ///
    /// # Panics
    ///
    /// Panics if the value fails to serialize.
    #[must_use]
    pub fn surrogate(value: &(impl Serialize + ?Sized)) -> Self {
        let json = serde_json::to_string(value).expect("failed to serialize surrogate value");
        Self(JsNode::Surrogate(json))
    }

    /// Returns every template this expression applies, including those of
    /// its holes.
    #[must_use]
    pub fn templates(&self) -> Vec<&'static JsTemplate> {
        let mut templates = Vec::new();
        let mut pending = vec![self];
        while let Some(js) = pending.pop() {
            if let JsNode::Template { template, holes } = &js.0 {
                templates.push(*template);
                pending.extend(holes);
            }
        }
        templates
    }

    /// Writes the JSON through `parts`, sealed for the writer's current
    /// context: a marker comment's body or a double-quoted attribute value.
    pub(crate) fn write(&self, parts: &mut PartsWriter<'_>) {
        match &self.0 {
            JsNode::Template { template, holes } => {
                parts
                    .push_promoted_str(&"[\"")
                    .push_static_str(template.id())
                    .push_promoted_str(&"\"");
                for hole in holes {
                    parts.push_promoted_str(&",");
                    hole.write(parts);
                }
                parts.push_promoted_str(&"]");
            }
            JsNode::Surrogate(json) => {
                parts
                    .push_promoted_str(&"{\"v\":")
                    .push_str(json)
                    .push_promoted_str(&"}");
            }
        }
    }
}

impl fmt::Display for Js {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            JsNode::Template { template, holes } => {
                write!(f, "[\"{}\"", template.id())?;
                for hole in holes {
                    write!(f, ",{hole}")?;
                }
                f.write_str("]")
            }
            JsNode::Surrogate(json) => write!(f, "{{\"v\":{json}}}"),
        }
    }
}

/// The JSON as the value of a `data-topcoat-*` attribute, like an event
/// handler or a bind expression.
impl AttributeValueViewParts for Js {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.write(parts);
    }
}

/// JavaScript source for an expression, known when the program is compiled.
///
/// Create and register one with [`js_template!`](crate::js_template). The
/// browser only runs templates included in its [`JsModule`](crate::JsModule).
#[derive(Debug)]
pub struct JsTemplate {
    id: [u8; 16],
    source: &'static str,
}

impl JsTemplate {
    /// Creates a template with an id derived from its source.
    ///
    /// The template is not registered. Use [`js_template!`](crate::js_template)
    /// to create one the browser can find.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(source: &'static str) -> Self {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";

        let hash = Fnv1a::<u64>::new().write(source.as_bytes()).finish();
        let mut id = [0; 16];
        let mut digit = 0;
        while digit < id.len() {
            id[digit] = DIGITS[((hash >> (60 - 4 * digit)) & 0xf) as usize];
            digit += 1;
        }
        Self { id, source }
    }

    /// The id the browser looks this template up by.
    #[must_use]
    pub fn id(&self) -> &str {
        match str::from_utf8(&self.id) {
            Ok(id) => id,
            Err(_) => unreachable!("template ids are hexadecimal"),
        }
    }

    /// The template's JavaScript source.
    #[must_use]
    pub fn source(&self) -> &'static str {
        self.source
    }
}

inventory::collect!(&'static JsTemplate);

/// Registers a [`JsTemplate`] with the [`JsModule`] of this binary and
/// returns the [`Js`] that applies it to the given holes.
///
/// The source must be a string literal, or another constant `&'static str`,
/// holding a JavaScript function expression. The browser runtime calls it
/// with its context, conventionally named `cx`, and an array of functions
/// that evaluate the holes. Each call evaluates its hole again.
///
/// ```rust
/// use topcoat::runtime::{Expr, JsModule, js_template};
///
/// let js = js_template!("(cx, holes) => !holes[0]()", Expr::from(true).js().clone());
/// let negated = Expr::evaluate(|| false, js);
///
/// let id = negated.js().templates()[0].id();
/// assert!(
///     JsModule::registered()
///         .templates()
///         .iter()
///         .any(|t| t.id() == id)
/// );
/// ```
///
/// [`JsModule`]: crate::JsModule
#[macro_export]
macro_rules! js_template {
    ($source:expr $(, $hole:expr)* $(,)?) => {{
        static TEMPLATE: $crate::JsTemplate = $crate::JsTemplate::new($source);
        $crate::internal::inventory::submit! { &TEMPLATE }
        $crate::Js::template(&TEMPLATE, ::std::vec![$($hole),*])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEMPLATE: JsTemplate = JsTemplate::new("(cx, holes) => holes[0]()");

    #[test]
    fn display_nests_holes_and_keeps_captured_json() {
        let inner = Js::template(&TEMPLATE, vec![Js::surrogate("\"<>&\n")]);
        let js = Js::template(&TEMPLATE, vec![inner]);
        let id = TEMPLATE.id();

        assert_eq!(
            js.to_string(),
            format!(r#"["{id}",["{id}",{{"v":"\"<>&\n"}}]]"#),
        );
    }
}
