import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// OTA Update state (in-memory)
// ──────────────────────────────────────────────
interface OTAUpdate {
  id: string;
  type: 'core-binary' | 'block-db' | 'ai-weights' | 'node-list';
  target: string;
  targetFa: string;
  version: string;
  currentVersion: string;
  size: number;
  deltaPatch: boolean;
  signature: string;
  sha256: string;
  status: 'available' | 'downloading' | 'installed' | 'failed';
  githubReleaseUrl: string;
  releaseNotesFa: string;
  priority: 'critical' | 'recommended' | 'optional';
  downloadProgress: number;
}

const otaUpdates: OTAUpdate[] = [
  {
    id: 'upd-1',
    type: 'core-binary',
    target: 'GFW-knocker/Xray-core',
    targetFa: 'ایکس‌ری GFW',
    version: 'v25.8.3-mahsa-r1',
    currentVersion: 'v25.8.3-mahsa-r1',
    size: 5200000,
    deltaPatch: true,
    signature: 'sha256:a1b2c3d4e5f6g7h8',
    sha256: 'a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0',
    status: 'installed',
    githubReleaseUrl: 'https://api.github.com/repos/GFW-knocker/Xray-core/releases/latest',
    releaseNotesFa: 'بهبود Fragment TLS و رفع اشکال دست‌دهی QUIC — نسخه اختصاصی مهسا',
    priority: 'recommended',
    downloadProgress: 100,
  },
  {
    id: 'upd-2',
    type: 'block-db',
    target: 'iran-block-signatures',
    targetFa: 'امضاهای مسدودیت ایران',
    version: '2026.05.23-r2',
    currentVersion: '2026.05.20-r1',
    size: 256000,
    deltaPatch: false,
    signature: 'sha256:f6e5d4c3b2a1',
    sha256: 'f6e5d4c3b2a1z0y9x8w7v6u5t4s3r2q1p0o9n8m7',
    status: 'available',
    githubReleaseUrl: '',
    releaseNotesFa: 'به‌روزرسانی امضاهای DPI — شناسایی الگوهای جدید فیلترینگ SNI و DNS — حیاتی برای عبور از فیلترینگ',
    priority: 'critical',
    downloadProgress: 0,
  },
  {
    id: 'upd-3',
    type: 'ai-weights',
    target: 'ucb-mab-model',
    targetFa: 'مدل هوش مصنوعی UCB',
    version: '3.1.0',
    currentVersion: '3.0.8',
    size: 1280000,
    deltaPatch: true,
    signature: 'sha256:1a2b3c4d5e6f',
    sha256: '1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t',
    status: 'available',
    githubReleaseUrl: '',
    releaseNotesFa: 'مدل جدید UCB-MAB با بهبود ۱۵٪ دقت پیش‌بینی مسدودسازی و تشخیص سریع‌تر ISP',
    priority: 'recommended',
    downloadProgress: 0,
  },
  {
    id: 'upd-4',
    type: 'node-list',
    target: 'hiddify-nodes',
    targetFa: 'لیست نودهای هیدیفای',
    version: '2026.05.23',
    currentVersion: '2026.05.22',
    size: 64000,
    deltaPatch: false,
    signature: 'sha256:9z8y7x6w5v4',
    sha256: '9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3j2i1h0g',
    status: 'installed',
    githubReleaseUrl: 'https://api.github.com/repos/hiddify/hiddify-core/releases/latest',
    releaseNotesFa: 'لیست نودهای به‌روزشده — حذف نودهای مسدود و اضافه شدن نودهای جدید اروپا',
    priority: 'optional',
    downloadProgress: 100,
  },
  {
    id: 'upd-5',
    type: 'core-binary',
    target: 'DefyxVPN',
    targetFa: 'دیفیکسوی‌پی‌ان',
    version: 'v5.3.0',
    currentVersion: 'v5.2.8',
    size: 3800000,
    deltaPatch: false,
    signature: 'sha256:d4e5f6g7h8i9',
    sha256: 'd4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3',
    status: 'available',
    githubReleaseUrl: 'https://api.github.com/repos/UnboundTechCo/defyxVPN/releases/latest',
    releaseNotesFa: 'بهبود سرعت اتصال ۲۰٪ و رفع مشکل اتصال در شبکه‌های همراه اول — پشتیبانی از پروتکل جدید',
    priority: 'recommended',
    downloadProgress: 0,
  },
];

const otaState = {
  lastCheck: Date.now() - 3600000,
  nextCheck: Date.now() + 18000000,
  autoUpdate: true,
  rollbackEnabled: true,
  sha256Verification: true,
  checkIntervalHours: 6,
  maxConcurrentDownloads: 2,
  wifiOnlyDownload: true,
  notifyOnUpdate: true,
};

const rollbackHistory = [
  {
    id: 'rb-1',
    updateId: 'upd-1',
    fromVersion: 'v25.8.2-mahsa-r1',
    toVersion: 'v25.8.3-mahsa-r1',
    rolledBackFrom: 'v25.8.4-mahsa-r1',
    timestamp: Date.now() - 86400000,
    reason: 'QUIC regression causing packet loss',
    reasonFa: 'بازگشت QUIC باعث از دست رفتن بسته‌ها شد',
    successful: true,
  },
];

// ──────────────────────────────────────────────
// GET /api/ota
// ──────────────────────────────────────────────
export async function GET() {
  const availableUpdates = otaUpdates.filter((u) => u.status === 'available');
  const installedUpdates = otaUpdates.filter((u) => u.status === 'installed');
  const downloadingUpdates = otaUpdates.filter((u) => u.status === 'downloading');

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    state: otaState,
    updates: otaUpdates,
    summary: {
      totalUpdates: otaUpdates.length,
      availableCount: availableUpdates.length,
      installedCount: installedUpdates.length,
      downloadingCount: downloadingUpdates.length,
      failedCount: otaUpdates.filter((u) => u.status === 'failed').length,
      totalAvailableSize: availableUpdates.reduce((sum, u) => sum + u.size, 0),
      criticalAvailable: availableUpdates.filter((u) => u.priority === 'critical').length,
    },
    availableUpdates: availableUpdates.map((u) => ({
      id: u.id,
      type: u.type,
      target: u.target,
      targetFa: u.targetFa,
      version: u.version,
      currentVersion: u.currentVersion,
      size: u.size,
      sizeHuman: u.size > 1000000 ? `${Math.round(u.size / 100000) / 10} MB` : `${Math.round(u.size / 1000)} KB`,
      priority: u.priority,
      releaseNotesFa: u.releaseNotesFa,
      deltaPatch: u.deltaPatch,
    })),
    rollbackHistory,
    config: {
      typeLabels: {
        'core-binary': { name: 'Core Binary', nameFa: 'باینری هسته', description: 'Executable core updates', descriptionFa: 'به‌روزرسانی فایل اجرایی هسته' },
        'block-db': { name: 'Block Database', nameFa: 'پایگاه داده مسدودیت', description: 'DPI signature and block list updates', descriptionFa: 'به‌روزرسانی امضاهای DPI و لیست مسدودیت' },
        'ai-weights': { name: 'AI Model Weights', nameFa: 'وزن‌های مدل هوش مصنوعی', description: 'Machine learning model updates', descriptionFa: 'به‌روزرسانی مدل یادگیری ماشین' },
        'node-list': { name: 'Node List', nameFa: 'لیست نودها', description: 'VPN server node configuration updates', descriptionFa: 'به‌روزرسانی پیکربندی نودهای سرور VPN' },
      },
    },
    meta: {
      endpoint: '/api/ota',
      descriptionFa: 'مدیریت به‌روزرسانی‌های OTA (Over-The-Air)',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/ota
// body: { action: string, ...params }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { action } = body as { action?: string };

    if (!action) {
      return NextResponse.json(
        {
          success: false,
          error: 'Missing required field: action',
          errorFa: 'فیلد ضروری موجود نیست: action',
        },
        { status: 400 },
      );
    }

    if (action === 'check-updates') {
      otaState.lastCheck = Date.now();
      otaState.nextCheck = Date.now() + otaState.checkIntervalHours * 3600000;

      // Simulate checking for updates
      const checkDuration = Math.round(1000 + Math.random() * 3000);

      return NextResponse.json({
        success: true,
        action: 'check-updates',
        checkDurationMs: checkDuration,
        lastCheck: otaState.lastCheck,
        nextCheck: otaState.nextCheck,
        availableCount: otaUpdates.filter((u) => u.status === 'available').length,
        criticalCount: otaUpdates.filter((u) => u.status === 'available' && u.priority === 'critical').length,
        message: `Update check complete — ${otaUpdates.filter((u) => u.status === 'available').length} update(s) available`,
        messageFa: `بررسی به‌روزرسانی کامل شد — ${otaUpdates.filter((u) => u.status === 'available').length} به‌روزرسانی موجود`,
      });
    }

    if (action === 'install-update') {
      const { updateId } = body as { updateId?: string };
      if (!updateId) {
        return NextResponse.json(
          {
            success: false,
            error: 'Missing required field: updateId',
            errorFa: 'فیلد ضروری موجود نیست: updateId',
          },
          { status: 400 },
        );
      }

      const update = otaUpdates.find((u) => u.id === updateId);
      if (!update) {
        return NextResponse.json(
          {
            success: false,
            error: `Update not found: ${updateId}. Valid IDs: ${otaUpdates.map((u) => u.id).join(', ')}`,
            errorFa: `به‌روزرسانی یافت نشد: ${updateId}`,
          },
          { status: 404 },
        );
      }

      if (update.status === 'installed') {
        return NextResponse.json({
          success: true,
          action: 'install-update',
          updateId,
          status: 'already-installed',
          message: `Update ${updateId} is already installed`,
          messageFa: `به‌روزرسانی ${updateId} از قبل نصب شده`,
        });
      }

      if (update.status === 'downloading') {
        return NextResponse.json({
          success: true,
          action: 'install-update',
          updateId,
          status: 'downloading',
          message: `Update ${updateId} is already downloading (${update.downloadProgress}%)`,
          messageFa: `به‌روزرسانی ${updateId} در حال دانلود است (${update.downloadProgress}٪)`,
        });
      }

      // Simulate download and install
      update.status = 'downloading';
      update.downloadProgress = 0;

      const installSuccess = Math.random() > 0.1;
      const downloadTime = Math.round(2000 + Math.random() * 5000);
      const installTime = Math.round(500 + Math.random() * 2000);

      // Save previous version BEFORE overwriting it
      const previousVersion = update.currentVersion;

      if (installSuccess) {
        update.status = 'installed';
        update.downloadProgress = 100;
        update.currentVersion = update.version;
      } else {
        update.status = 'failed';
        update.downloadProgress = 0;
      }

      return NextResponse.json({
        success: installSuccess,
        action: 'install-update',
        updateId,
        updateType: update.type,
        target: update.target,
        targetFa: update.targetFa,
        previousVersion,
        newVersion: update.version,
        downloadTimeMs: downloadTime,
        installTimeMs: installTime,
        sha256Verified: otaState.sha256Verification,
        status: update.status,
        message: installSuccess
          ? `Update ${update.target} (${update.version}) installed successfully`
          : `Update ${update.target} (${update.version}) installation failed`,
        messageFa: installSuccess
          ? `به‌روزرسانی ${update.targetFa} (${update.version}) با موفقیت نصب شد`
          : `نصب به‌روزرسانی ${update.targetFa} (${update.version}) ناموفق بود`,
      });
    }

    if (action === 'rollback') {
      const { updateId } = body as { updateId?: string };
      if (!updateId) {
        return NextResponse.json(
          {
            success: false,
            error: 'Missing required field: updateId',
            errorFa: 'فیلد ضروری موجود نیست: updateId',
          },
          { status: 400 },
        );
      }

      const update = otaUpdates.find((u) => u.id === updateId);
      if (!update) {
        return NextResponse.json(
          {
            success: false,
            error: `Update not found: ${updateId}`,
            errorFa: `به‌روزرسانی یافت نشد: ${updateId}`,
          },
          { status: 404 },
        );
      }

      if (!otaState.rollbackEnabled) {
        return NextResponse.json(
          {
            success: false,
            error: 'Rollback is disabled in settings',
            errorFa: 'بازگشت به نسخه قبلی در تنظیمات غیرفعال است',
          },
          { status: 400 },
        );
      }

      if (update.status !== 'installed') {
        return NextResponse.json(
          {
            success: false,
            error: `Cannot rollback update that is not installed (current status: ${update.status})`,
            errorFa: `نمی‌توان به‌روزرسانی نصب‌نشده را بازگرداند (وضعیت فعلی: ${update.status})`,
          },
          { status: 400 },
        );
      }

      // Perform rollback
      const rolledBackVersion = update.currentVersion;
      // previousVersion: for updates where version === currentVersion (already installed at latest),
      // derive a meaningful prior version label; otherwise use the older currentVersion.
      const previousVersion = update.version !== update.currentVersion
        ? update.currentVersion
        : `${update.version}-rollback`;
      update.status = 'available';
      update.downloadProgress = 0;

      const rollbackEntry = {
        id: `rb-${Date.now()}`,
        updateId,
        fromVersion: rolledBackVersion,
        toVersion: previousVersion,
        rolledBackFrom: update.version,
        timestamp: Date.now(),
        reason: 'Manual rollback via API',
        reasonFa: 'بازگشت دستی از طریق API',
        successful: true,
      };
      rollbackHistory.push(rollbackEntry);

      return NextResponse.json({
        success: true,
        action: 'rollback',
        updateId,
        rolledBackFrom: update.version,
        rolledBackTo: previousVersion,
        rollbackTimeMs: Math.round(800 + Math.random() * 1500),
        rollbackEntry,
        message: `Rollback successful — ${update.target} reverted from ${update.version} to ${previousVersion}`,
        messageFa: `بازگشت موفق — ${update.targetFa} از ${update.version} به ${previousVersion} بازگردانده شد`,
      });
    }

    if (action === 'update-settings') {
      const {
        autoUpdate,
        rollbackEnabled,
        sha256Verification,
        checkIntervalHours,
        wifiOnlyDownload,
        notifyOnUpdate,
      } = body as {
        autoUpdate?: boolean;
        rollbackEnabled?: boolean;
        sha256Verification?: boolean;
        checkIntervalHours?: number;
        wifiOnlyDownload?: boolean;
        notifyOnUpdate?: boolean;
      };

      if (typeof autoUpdate === 'boolean') otaState.autoUpdate = autoUpdate;
      if (typeof rollbackEnabled === 'boolean') otaState.rollbackEnabled = rollbackEnabled;
      if (typeof sha256Verification === 'boolean') otaState.sha256Verification = sha256Verification;
      if (typeof checkIntervalHours === 'number' && checkIntervalHours >= 1 && checkIntervalHours <= 72) {
        otaState.checkIntervalHours = checkIntervalHours;
      }
      if (typeof wifiOnlyDownload === 'boolean') otaState.wifiOnlyDownload = wifiOnlyDownload;
      if (typeof notifyOnUpdate === 'boolean') otaState.notifyOnUpdate = notifyOnUpdate;

      return NextResponse.json({
        success: true,
        action: 'update-settings',
        state: otaState,
        message: 'OTA settings updated',
        messageFa: 'تنظیمات OTA به‌روز شد',
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: check-updates, install-update, rollback, update-settings`,
        errorFa: `عملیات ناشناخته: ${action}`,
      },
      { status: 400 },
    );
  } catch (error) {
    return NextResponse.json(
      {
        success: false,
        error: 'Invalid JSON body',
        errorFa: 'بدنه JSON نامعتبر است',
        details: error instanceof Error ? error.message : 'Unknown error',
      },
      { status: 400 },
    );
  }
}
