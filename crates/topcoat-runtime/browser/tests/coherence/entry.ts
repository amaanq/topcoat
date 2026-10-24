import {
	compile as compileExpression,
	defineTemplates,
	type Expression,
	type Template,
} from "../../src/expression/compile";
import { SignalRegistry } from "../../src/signal-registry";
import { Panic } from "../../src/surrogate/panic";
import { FixtureContext } from "./fixture";
import { observe } from "./observe";

export { setServerUsizeBits } from "../../src/surrogate/integer";

/**
 * Executes production-generated JSON against the definitions of a template
 * module in an independent runtime context.
 */
export function execute(
	definitions: string,
	json: string,
	invoke: boolean,
): string {
	const cx = new FixtureContext(new SignalRegistry());
	const compute = compile(definitions, json);
	if (typeof compute === "string") return compute;
	let value: unknown;
	try {
		value = compute(cx);
		if (invoke) {
			if (typeof value !== "function") {
				throw new Error("Expected a compiled closure");
			}
			value = value();
		}
	} catch (error) {
		if (error instanceof Panic) {
			return JSON.stringify({ kind: "Panic", value: error.message });
		}
		if (error instanceof Error) {
			return JSON.stringify({ kind: "Exception", value: error.toString() });
		}
		throw error;
	}
	return JSON.stringify({ kind: "Return", value: observe(value) });
}

/** Invokes an async closure and observes its completed value. */
export async function executeAsync(
	definitions: string,
	json: string,
): Promise<string> {
	const cx = new FixtureContext(new SignalRegistry());
	const compute = compile(definitions, json);
	if (typeof compute === "string") return compute;
	let value: unknown;
	try {
		const run: unknown = compute(cx);
		if (typeof run !== "function") {
			throw new Error("Expected a compiled async closure");
		}
		const pending: unknown = run();
		if (!(pending instanceof Promise)) {
			throw new Error("Compiled async closure must return a promise");
		}
		value = await pending;
	} catch (error) {
		if (error instanceof Panic) {
			return JSON.stringify({ kind: "Panic", value: error.message });
		}
		if (error instanceof Error) {
			return JSON.stringify({ kind: "Exception", value: error.toString() });
		}
		throw error;
	}
	return JSON.stringify({ kind: "Return", value: observe(value) });
}

/** Loads the module's definitions as a page would, then resolves the JSON. */
function compile(definitions: string, json: string): Expression | string {
	let templates: { [id: string]: Template };
	try {
		templates = new Function(`return ${definitions};`)() as {
			[id: string]: Template;
		};
	} catch (error) {
		if (error instanceof SyntaxError) {
			return JSON.stringify({ kind: "CompileError", value: error.toString() });
		}
		throw error;
	}
	defineTemplates(templates);
	return compileExpression(json, "coherence case");
}
