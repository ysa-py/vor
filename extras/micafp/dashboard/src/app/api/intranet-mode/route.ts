// MICAFP UnifiedShield VIP-ULTRA — NAIN Intranet Mode API Route
// Exposes NAIN status and channel activation controls to the dashboard.

import { NextRequest, NextResponse } from 'next/server';

export async function GET() {
  return NextResponse.json({
    nain_active: false,
    active_channels: [],
    ble_mesh_enabled: false,
    wifi_aware_enabled: false,
    sms_bootstrap_enabled: false,
    acoustic_covert_enabled: false,
    ntp_covert_enabled: false,
  });
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const { channel, enabled } = body as { channel: string; enabled: boolean };
  // In production: send IPC command to daemon
  return NextResponse.json({ ok: true, channel, enabled });
}
