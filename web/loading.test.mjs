import assert from "node:assert/strict";
import test from "node:test";
import { fetchWasm } from "./loading.mjs";

function resources(size, chunks) {
    return async path => path.endsWith(".json")
        ? Response.json({ bytes: size })
        : new Response(new ReadableStream({
            pull(controller) {
                if (chunks.length) controller.enqueue(new Uint8Array(chunks.shift()));
                else controller.close();
            },
        }), { headers: { "Content-Type": "application/wasm", "Content-Encoding": "gzip" } });
}

test("tracks decoded download bytes while preserving streaming", async () => {
    const progress = [];
    const response = await fetchWasm(
        (loaded, total) => progress.push([loaded, total]),
        resources(5, [[1, 2], [3, 4, 5]]),
    );
    assert.equal(response.headers.get("Content-Type"), "application/wasm");
    assert.deepEqual([...new Uint8Array(await response.arrayBuffer())], [1, 2, 3, 4, 5]);
    assert.deepEqual(progress, [[2, 5], [5, 5]]);
});

test("rejects a truncated or oversized download", async () => {
    for (const [size, chunks] of [[5, [[1, 2]]], [1, [[1, 2]]]]) {
        const response = await fetchWasm(() => {}, resources(size, chunks));
        await assert.rejects(response.arrayBuffer(), /download size does not match/);
    }
});

test("reports invalid metadata and failed downloads", async () => {
    await assert.rejects(fetchWasm(() => {}, resources(0, [])), /Invalid WASM size/);
    await assert.rejects(fetchWasm(() => {}, async path => path.endsWith(".json")
        ? Response.json({ bytes: 1 })
        : new Response(null, { status: 404 })), /Could not load WASM: HTTP 404/);
    await assert.rejects(fetchWasm(() => {}, async path => path.endsWith(".json")
        ? new Response(null, { status: 404 })
        : new Response(new Uint8Array([0]))), /Could not load WASM size: HTTP 404/);
});
