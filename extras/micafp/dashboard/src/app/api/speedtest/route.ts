import { NextRequest, NextResponse } from 'next/server';

/**
 * Speed-test endpoint used by the real network-stats service.
 *
 * - GET  ?bytes=N  streams exactly N bytes (cap 10 MB) with a real
 *       Content-Length so the browser can compute Mbps = (bytes*8)/(sec*1e6).
 * - POST            echoes back the received byte count so upload throughput
 *       can be measured the same way.
 *
 * This measures genuine throughput between the client and this server,
 * which is the correct source for a live network-stats readout.
 */
export const dynamic = 'force-dynamic';

const MAX_BYTES = 10_000_000; // 10 MB

export async function GET(request: NextRequest) {
  const bytesParam = Number(request.nextUrl.searchParams.get('bytes') ?? 5_000_000);
  const bytes = Math.min(Math.max(1_000_000, bytesParam), MAX_BYTES);

  // Deterministic pseudo-random payload (fast to generate, no crypto overhead)
  const chunk = new Uint8Array(65536);
  let seed = 0x9e3779b9;
  for (let i = 0; i < chunk.length; i++) {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    chunk[i] = seed & 0xff;
  }

  const response = new NextResponse(new ReadableStream({
    start(controller) {
      let sent = 0;
      const push = () => {
        while (sent < bytes) {
          const remaining = bytes - sent;
          const len = Math.min(chunk.length, remaining);
          controller.enqueue(chunk.subarray(0, len));
          sent += len;
        }
        controller.close();
      };
      push();
    },
  }), {
    headers: {
      'Content-Type': 'application/octet-stream',
      'Content-Length': String(bytes),
      'Cache-Control': 'no-store, max-age=0',
      'Access-Control-Allow-Origin': '*',
    },
  });

  return response;
}

export async function POST(request: NextRequest) {
  const contentType = request.headers.get('content-type') ?? '';
  let received = 0;

  if (contentType.includes('application/json')) {
    // JSON upload: measure the exact serialized payload size
    const text = await request.text();
    received = Buffer.byteLength(text, 'utf8');
  } else {
    // Raw body upload
    const buf = await request.arrayBuffer();
    received = buf.byteLength;
  }

  return NextResponse.json({ received, at: Date.now() });
}
