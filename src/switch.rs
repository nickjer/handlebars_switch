use handlebars::{
    BlockContext, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, RenderErrorReason, Renderable,
};

use serde_json::{Value, json};

struct StringBuffer(String);

impl Output for StringBuffer {
    fn write(&mut self, seg: &str) -> Result<(), std::io::Error> {
        self.0.push_str(seg);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct DefaultHelper;

impl HelperDef for DefaultHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        _out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(block) = rc.block_mut()
            && block.get_local_var("default_output").is_some()
        {
            return Err(RenderErrorReason::Other(
                "Multiple {{#default}} blocks in a single {{#switch}}".to_string(),
            )
            .into());
        }

        let mut buf = StringBuffer(String::new());
        if let Some(t) = h.template() {
            t.render(r, ctx, rc, &mut buf)?;
        }

        if let Some(block) = rc.block_mut() {
            block.set_local_var("default_output", json!(buf.0));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct CaseHelper {
    expression_value: serde_json::Value,
}

impl HelperDef for CaseHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(block) = rc.block_mut() {
            let prev_found = block
                .get_local_var("match")
                .and_then(Value::as_bool)
                .unwrap_or_default();
            if !prev_found
                && h.params()
                    .iter()
                    .any(|x| *x.value() == self.expression_value)
            {
                // found match
                block.set_local_var("match", json!(true));
                match h.template() {
                    Some(t) => t.render(r, ctx, rc, out),
                    None => Ok(()),
                }
            } else {
                // did not find match
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

/// Switch Helper
///
/// Provides the `{{#switch}}` helper to a Handlebars template.
///
/// # Examples
///
/// ```
/// use handlebars::Handlebars;
/// use handlebars_switch::SwitchHelper;
/// use serde_json::json;
///
/// let mut handlebars = Handlebars::new();
/// handlebars.register_helper("switch", Box::new(SwitchHelper));
///
/// let tpl = "\
///     {{#switch access}}\
///         {{#case \"admin\"}}Admin{{/case}}\
///         {{#default}}User{{/default}}\
///     {{/switch}}\
/// ";
///
/// assert_eq!(
///     handlebars.render_template(tpl, &json!({"access": "admin"})).unwrap(),
///     "Admin"
/// );
///
/// assert_eq!(
///     handlebars.render_template(tpl, &json!({"access": "nobody"})).unwrap(),
///     "User"
/// );
/// ```
#[derive(Clone, Copy)]
pub struct SwitchHelper;

impl HelperDef for SwitchHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Read in the switch variable or expression
        let param = h
            .param(0)
            .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("switch", 0))?;

        let expression_value = param.value().clone();

        // Keep track of whether a match occurs within the block
        let mut block_context = BlockContext::new();
        block_context.set_local_var("match", json!(false));
        let mut local_rc = rc.clone();
        local_rc.push_block(block_context);

        // Add the `{{#case}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper("case", Box::new(CaseHelper { expression_value }));

        // Add the `{{#default}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper("default", Box::new(DefaultHelper));

        // Render the `{{#switch}}` block
        let result = match h.template() {
            Some(t) => t.render(r, ctx, &mut local_rc, out),
            None => Ok(()),
        };

        // Emit buffered default output if no case matched
        let result = result.and_then(|()| {
            let matched = local_rc
                .block()
                .and_then(|b| b.get_local_var("match"))
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if !matched {
                if let Some(output) = local_rc
                    .block()
                    .and_then(|b| b.get_local_var("default_output"))
                    .and_then(Value::as_str)
                {
                    out.write(output).map_err(RenderError::from)
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        });

        local_rc.pop_block();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::SwitchHelper;
    use handlebars::Handlebars;
    use serde_json::json;

    #[test]
    fn test_switch() {
        let tpl = "\
            {{#switch state}}\
                {{#case \"page1\" \"page2\"}}\
                    page 1 or 2\
                    {{#switch s}}\
                        {{#case 4}}s = 4{{/case}}\
                    {{/switch}}\
                {{/case}}\
                {{#case \"page3\"}}page3{{/case}}\
                {{#case \"page4\"}}page4{{/case}}\
                {{#case \"page5\"}}\
                    page5 - \
                    {{#switch s}}\
                        {{#case 3}}s = 3{{/case}}\
                        {{#case 2}}s = 2{{/case}}\
                        {{#case 1}}s = 1{{/case}}\
                        {{#default}}unknown{{/default}}\
                    {{/switch}}\
                {{/case}}\
                {{#default}}page0{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));
        assert!(handlebars.register_template_string("tpl", tpl).is_ok());

        let r0 = handlebars.render("tpl", &json!({"state": "page2", "s": 1}));
        assert_eq!(r0.ok().unwrap(), "page 1 or 2");

        let r1 = handlebars.render("tpl", &json!({"state": "page5", "s": 1}));
        assert_eq!(r1.ok().unwrap(), "page5 - s = 1");

        let r2 = handlebars.render("tpl", &json!({"state": "page5", "s": 4}));
        assert_eq!(r2.ok().unwrap(), "page5 - unknown");

        let r3 = handlebars.render("tpl", &json!({"state": "page0", "s": 1}));
        assert_eq!(r3.ok().unwrap(), "page0");
    }

    #[test]
    fn test_missing_key_renders_default() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(handlebars.render_template(tpl, &json!({})).unwrap(), "User");
    }

    #[test]
    fn test_case_helper_not_defined() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
            {{#case \"test\"}}Check{{/case}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .is_err()
        );
    }

    #[test]
    fn test_default_helper_not_defined() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
            {{#default \"test\"}}Check{{/default}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .is_err()
        );
    }

    #[test]
    fn test_only_case_exists_with_match() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .unwrap(),
            "Admin"
        );
    }

    #[test]
    fn test_only_case_exists_with_no_match() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "unknown"}))
                .unwrap(),
            ""
        );
    }

    #[test]
    fn test_only_default_exists() {
        let tpl = "\
            {{#switch access}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .unwrap(),
            "User"
        );
    }

    #[test]
    fn test_switch_on_integer() {
        let tpl = "\
            {{#switch count}}\
                {{#case 1}}one{{/case}}\
                {{#case 2}}two{{/case}}\
                {{#default}}other{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"count": 1}))
                .unwrap(),
            "one"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"count": 2}))
                .unwrap(),
            "two"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"count": 99}))
                .unwrap(),
            "other"
        );
    }

    #[test]
    fn test_switch_on_boolean() {
        let tpl = "\
            {{#switch active}}\
                {{#case true}}yes{{/case}}\
                {{#case false}}no{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"active": true}))
                .unwrap(),
            "yes"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"active": false}))
                .unwrap(),
            "no"
        );
    }

    #[test]
    fn test_multiple_defaults_is_error() {
        let tpl = "\
            {{#switch access}}\
                {{#default}}first{{/default}}\
                {{#default}}second{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert!(
            handlebars
                .render_template(tpl, &json!({"access": "nobody"}))
                .is_err()
        );
    }

    #[test]
    fn test_default_before_case_with_match() {
        let tpl = "\
            {{#switch access}}\
                {{#default}}User{{/default}}\
                {{#case \"admin\"}}Admin{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .unwrap(),
            "Admin"
        );
    }

    #[test]
    fn test_default_before_case_without_match() {
        let tpl = "\
            {{#switch access}}\
                {{#default}}User{{/default}}\
                {{#case \"admin\"}}Admin{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "nobody"}))
                .unwrap(),
            "User"
        );
    }

    #[test]
    fn test_empty_switch_body() {
        let tpl = "{{#switch access}}{{/switch}}";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"access": "admin"}))
                .unwrap(),
            ""
        );
    }

    #[test]
    fn test_switch_on_null() {
        let tpl = "\
            {{#switch missing}}\
                {{#case \"something\"}}found{{/case}}\
                {{#default}}fallback{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"missing": null}))
                .unwrap(),
            "fallback"
        );
    }

    #[test]
    fn test_nested_switch_in_default() {
        let tpl = "\
            {{#switch outer}}\
                {{#case \"a\"}}A{{/case}}\
                {{#default}}\
                    {{#switch inner}}\
                        {{#case \"x\"}}X{{/case}}\
                        {{#default}}fallback{{/default}}\
                    {{/switch}}\
                {{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "a", "inner": "x"}))
                .unwrap(),
            "A"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "b", "inner": "x"}))
                .unwrap(),
            "X"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "b", "inner": "y"}))
                .unwrap(),
            "fallback"
        );
    }

    #[test]
    fn test_nested_defaults_are_independent() {
        let tpl = "\
            {{#switch outer}}\
                {{#default}}\
                    {{#switch inner}}\
                        {{#default}}inner-default{{/default}}\
                    {{/switch}}\
                {{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "x", "inner": "y"}))
                .unwrap(),
            "inner-default"
        );
    }

    #[test]
    fn test_nested_default_before_case() {
        let tpl = "\
            {{#switch outer}}\
                {{#default}}\
                    {{#switch inner}}\
                        {{#default}}inner-default{{/default}}\
                        {{#case \"x\"}}X{{/case}}\
                    {{/switch}}\
                {{/default}}\
                {{#case \"a\"}}A{{/case}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "a", "inner": "x"}))
                .unwrap(),
            "A"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "b", "inner": "x"}))
                .unwrap(),
            "X"
        );

        assert_eq!(
            handlebars
                .render_template(tpl, &json!({"outer": "b", "inner": "y"}))
                .unwrap(),
            "inner-default"
        );
    }
}
