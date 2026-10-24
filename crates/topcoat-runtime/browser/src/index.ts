import { defineTemplates, type Template } from "./expression/compile";
import { Runtime } from "./runtime";

const runtime = new Runtime();

// The server's `<script>` for the template module runs before this one, so
// importing it again reuses the evaluated module. If it fails to load, the
// runtime still starts and each expression reports its own error.
const templates = document.querySelector<HTMLScriptElement>(
	"script[data-topcoat-expressions]",
);
if (templates !== null) {
	try {
		const module = (await import(templates.src)) as {
			default: { [id: string]: Template };
		};
		defineTemplates(module.default);
	} catch (error) {
		runtime.reportError(error);
	}
}

runtime.start(document);
runtime.page.listenForDevRefresh();
