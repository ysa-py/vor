Name:           unifiedshield
Version:        1.0.0
Release:        1%{?dist}
Summary:        Next-gen anti-censorship VPN for Iran

License:        GPL-3.0
URL:            https://github.com/unifiedshield/unifiedshield
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cmake >= 3.22
BuildRequires:  gcc-c++
BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig
BuildRequires:  systemd-rpm-macros

Requires:       openssl-libs
Requires:       systemd
Requires:       %{name}-core = %{version}-%{release}

%description
UnifiedShield is a next-generation anti-censorship VPN client designed
specifically for users in Iran. It supports multiple protocol cores
(Xray, NaïveProxy, Hysteria2, TUIC) with automatic DPI evasion.

Key features:
- Split tunnel (Iranian IPs excluded for local banking/gov access)
- DPI detection with sliding window algorithm (threshold > 0.72)
- Kill switch (always-on VPN lockdown)
- DNS: Alibaba/Tencent CDN primary (Cloudflare blocked in Iran)
- No root required (uses CAP_NET_ADMIN capability)
- OTA updates via GitHub Releases with SHA256 verification

%prep
%autosetup

%build
# Build Rust core
cargo build --release --manifest-path ../Cargo.toml

# Build C++ wrapper
%cmake -DCMAKE_BUILD_TYPE=Release
%cmake_build

%install
%cmake_install

# Install systemd service
install -Dm644 linux/unifiedshield.service \
    %{buildroot}%{_unitdir}/unifiedshield.service

# Install config directory
install -dm755 %{buildroot}%{_sysconfdir}/unifiedshield
install -Dm600 /dev/null %{buildroot}%{_sysconfdir}/unifiedshield/config.json

# Install runtime directory
install -dm755 %{buildroot}%{_rundir}/unifiedshield

%post
%systemd_post unifiedshield.service

%preun
%systemd_preun unifiedshield.service

%postun
%systemd_postun_with_restart unifiedshield.service

%files
%license LICENSE
%doc README.md
%{_bindir}/unifiedshield
%{_unitdir}/unifiedshield.service
%dir %{_sysconfdir}/unifiedshield
%config(noreplace) %{_sysconfdir}/unifiedshield/config.json
%dir %attr(0755,root,root) %{_rundir}/unifiedshield

%changelog
* Mon Jan 01 2024 UnifiedShield Team <team@unifiedshield.org> - 1.0.0-1
- Initial RPM package
- Multi-core support: Xray, NaïveProxy, Hysteria2, TUIC
- DPI detection with automatic core switching
- Split tunnel for Iranian IPs
- Kill switch support
- Chinese CDN DNS (Alibaba/Tencent)
