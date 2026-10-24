import { defineTemplates, type Template } from "../src/expression/compile";

/**
 * Defines a template that evaluates `source` and returns the JSON applying
 * it, escaped for a double-quoted attribute value or a marker comment.
 */
export function js(source: string): string {
	defineTemplates({
		[source]: new Function("cx", `return (${source});`) as Template,
	});
	return JSON.stringify([source])
		.replaceAll("&", "&amp;")
		.replaceAll('"', "&quot;");
}
