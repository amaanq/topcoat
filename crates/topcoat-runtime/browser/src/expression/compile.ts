import type { Context } from "./context";
import type { DehydratedSurrogate } from "./serialized";

export type Expression<T = unknown> = (cx: Context) => T;

/**
 * A function from the server's template module. Each hole evaluates one of
 * the expressions the server captured into the template.
 */
export type Template = (
	cx: Context,
	holes: readonly (() => unknown)[],
) => unknown;

const templates = new Map<string, Template>();

/** Adds the templates exported by the server's template module. */
export function defineTemplates(definitions: {
	readonly [id: string]: Template;
}): void {
	for (const [id, template] of Object.entries(definitions)) {
		templates.set(id, template);
	}
}

/**
 * Resolves server-generated JSON against the defined templates without
 * evaluating it. The JSON is either a template id followed by its holes,
 * `["<id>", ...holes]`, or a captured value, `{"v": <value>}`.
 */
export function compile<T = unknown>(
	json: string,
	label: string,
): Expression<T> {
	try {
		return link(JSON.parse(json) as unknown) as Expression<T>;
	} catch (cause) {
		throw new Error(`Failed to compile ${label}`, { cause });
	}
}

function link(plan: unknown): Expression {
	if (Array.isArray(plan)) {
		const [id, ...rest] = plan as unknown[];
		const template = typeof id === "string" ? templates.get(id) : undefined;
		if (template === undefined) {
			throw new Error(`Unknown expression template: ${String(id)}`);
		}
		const holes = rest.map(link);
		// The runtime passes the same context to every evaluation.
		let context: Context | null = null;
		let thunks: (() => unknown)[] = [];
		return (cx) => {
			if (cx !== context) {
				context = cx;
				thunks = holes.map((hole) => () => hole(cx));
			}
			return template(cx, thunks);
		};
	}
	if (
		typeof plan === "object" &&
		plan !== null &&
		Object.keys(plan).length === 1 &&
		"v" in plan
	) {
		const value = (plan as { v: DehydratedSurrogate }).v;
		return (cx) => cx.hydrate(value);
	}
	throw new Error("Invalid expression");
}
