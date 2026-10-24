import { compile } from "../expression/compile";
import type { Scope } from "../scope";
import { Event } from "../surrogate";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

/** The synthetic event fired each time an element enters the viewport. */
const VISIBLE_EVENT = "visible";

type EventHandler = (event: unknown) => void;

let viewport: IntersectionObserver | null = null;

function observeVisible(el: Element, lifetime: AbortSignal): void {
	if (typeof IntersectionObserver === "undefined" || lifetime.aborted) return;
	viewport ??= new IntersectionObserver((entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) {
				entry.target.dispatchEvent(new CustomEvent(VISIBLE_EVENT));
			}
		}
	});
	const observer = viewport;
	observer.observe(el);
	lifetime.addEventListener("abort", () => observer.unobserve(el), {
		once: true,
	});
}

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const expression = compile<EventHandler>(attr.value, `event @${name}`);
	const handler = expression(scope.runtime.context);
	// Dispose the listener with its scope. A DOM update may reuse the element
	// and attach a new listener.
	el.addEventListener(name, (event) => handler(new Event(event)), {
		signal: scope.abortSignal,
	});
	if (name === VISIBLE_EVENT) observeVisible(el, scope.abortSignal);
}
