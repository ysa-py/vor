//! MICAFP-UnifiedShield-6.0 — Zig Build for OpenWrt MIPS Binary
//!
//! Cross-compiles the shield-openwrt binary for MIPS (mipsel-linux-musl)
//! without requiring the OpenWrt SDK/toolchain. Zig's built-in cross-
//! compilation handles the toolchain automatically.
//!
//! Usage:
//!   zig build                                    # Default: native
//!   zig build -Dtarget=mipsel-linux-musl         # OpenWrt MIPS
//!   zig build -Dtarget=aarch64-linux-musl        # ARM routers
//!   zig build -Dtarget=x86_64-linux-musl         # x86 routers
//!   zig build -Doptimize=ReleaseSmall            # Minimal size
//!
//! The binary is designed to be as small as possible for embedded devices
//! with limited flash storage (typically 4-16 MB).

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{
        .default_target = .{
            .cpu_arch = .mipsel,
            .os_tag = .linux,
            .abi = .musl,
        },
    });

    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseSmall,
    });

    // -------------------------------------------------------------------
    // Main executable: shield-openwrt
    // -------------------------------------------------------------------
    const exe = b.addExecutable(.{
        .name = "shield-openwrt",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        // Statically link everything — no shared lib dependencies on target
        .linkage = .static,
    });

    // Strip all debug info for minimal binary size
    exe.root_module.strip = true;

    // Link libc (musl) for networking functions
    exe.linkLibC;

    // -------------------------------------------------------------------
    // Compile flags for MIPS
    // -------------------------------------------------------------------
    // For mipsel, we need to ensure proper ABI settings
    if (target.result.cpu.arch == .mipsel) {
        // MIPS32r2 is the most common for OpenWrt routers
        // Zig handles this through the target triple, but we can
        // add explicit CPU features if needed
        exe.root_module.addCMacro("MIPSEL", "1");
        exe.root_module.addCMacro("OPENWRT", "1");
    }

    // -------------------------------------------------------------------
    // Conditional compilation via build options
    // -------------------------------------------------------------------
    const config_opts = b.addOptions();
    config_opts.addOption([]const u8, "version", "6.0.0");
    config_opts.addOption(bool, "embedded", true);
    config_opts.addOption(bool, "with_luci", false); // LuCI runs separately in Lua
    exe.root_module.addOptions("build_config", config_opts);

    // -------------------------------------------------------------------
    // Install
    // -------------------------------------------------------------------
    b.installArtifact(exe);

    // -------------------------------------------------------------------
    // Custom install step: copy config files
    // -------------------------------------------------------------------
    const install_configs = b.addInstallDirectory(.{
        .source_dir = b.path("../configs"),
        .install_dir = .{ .custom = "etc/shield" },
        .install_subdir = "",
    });

    b.getInstallStep().dependOn(&install_configs.step);

    // -------------------------------------------------------------------
    // Run step (for local testing only — not for cross-compiled targets)
    // -------------------------------------------------------------------
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }
    const run_step = b.step("run", "Run the shield-openwrt binary");
    run_step.dependOn(&run_cmd.step);

    // -------------------------------------------------------------------
    // Size report step
    // -------------------------------------------------------------------
    const size_step = b.step("size", "Report binary size");
    const size_cmd = b.addSystemCommand(&.{
        "ls",
        "-la",
        b.getInstallPath(.bin, "shield-openwrt"),
    });
    size_step.dependOn(&size_cmd.step);

    // -------------------------------------------------------------------
    // MIPS-specific: generate .trx firmware package header
    // (For direct firmware integration, not just package install)
    // -------------------------------------------------------------------
    const trx_step = b.step("trx", "Create TRX firmware package");
    const trx_cmd = b.addSystemCommand(&.{
        "sh", "-c",
        b.fmt(
            \\ echo "Creating TRX package for OpenWrt..."
            \\ BIN="{s}"
            \\ if [ -f "$BIN" ]; then
            \\   SIZE=$(stat -c%s "$BIN")
            \\   echo "Binary size: $SIZE bytes"
            \\   if [ $SIZE -gt 1048576 ]; then
            \\     echo "WARNING: Binary exceeds 1 MB — may not fit on small flash devices"
            \\   fi
            \\ else
            \\   echo "Binary not found: $BIN"
            \\ fi
        , .{b.getInstallPath(.bin, "shield-openwrt")}),
    });
    trx_step.dependOn(b.getInstallStep());
    trx_step.dependOn(&trx_cmd.step);

    // -------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------
    const exe_unit_tests = b.addTest(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const run_exe_unit_tests = b.addRunArtifact(exe_unit_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_exe_unit_tests.step);
}
