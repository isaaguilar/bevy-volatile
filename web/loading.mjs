export async function fetchWasm(onProgress, fetchResource = fetch) {
    const [sizeResponse, wasmResponse] = await Promise.all([
        fetchResource("./wasm-size.json"),
        fetchResource("./volatile_bg.wasm"),
    ]);
    if (!sizeResponse.ok) {
        throw new Error(`Could not load WASM size: HTTP ${sizeResponse.status}`);
    }
    if (!wasmResponse.ok) {
        throw new Error(`Could not load WASM: HTTP ${wasmResponse.status}`);
    }

    const { bytes: total } = await sizeResponse.json();
    if (!Number.isSafeInteger(total) || total <= 0) {
        throw new Error("Invalid WASM size");
    }
    if (!wasmResponse.body) {
        const bytes = await wasmResponse.arrayBuffer();
        if (bytes.byteLength !== total) throw new Error("WASM download size does not match");
        onProgress(total, total);
        return bytes;
    }

    const reader = wasmResponse.body.getReader();
    let loaded = 0;
    const body = new ReadableStream({
        async pull(controller) {
            try {
                const { done, value } = await reader.read();
                if (done) {
                    if (loaded !== total) throw new Error("WASM download size does not match");
                    controller.close();
                } else {
                    loaded += value.byteLength;
                    if (loaded > total) throw new Error("WASM download size does not match");
                    onProgress(loaded, total);
                    controller.enqueue(value);
                }
            } catch (error) {
                controller.error(error);
            }
        },
        cancel(reason) {
            return reader.cancel(reason);
        },
    });
    return new Response(body, {
        headers: { "Content-Type": wasmResponse.headers.get("Content-Type") || "application/octet-stream" },
    });
}
