cmake_minimum_required(VERSION 3.22)
project(UnifiedShield VERSION 1.0.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find Rust static library
set(RUST_LIB_DIR "${CMAKE_SOURCE_DIR}/../target/release")
set(RUST_LIB_NAME "unifiedshield")

# Find required packages
find_package(Threads REQUIRED)
find_package(PkgConfig REQUIRED)

# TUN device support
pkg_check_modules(TUN REQUIRED IMPORTED_TARGET net)

# Source files
set(SOURCES
    src/main.cpp
    src/tun_interface.cpp
    src/systemd_service.cpp
    src/ipc_client.cpp
)

# Create executable
add_executable(unifiedshield ${SOURCES})

# Link libraries
target_link_libraries(unifiedshield
    PRIVATE
    Threads::Threads
    PkgConfig::TUN
    ${RUST_LIB_DIR}/${CMAKE_STATIC_LIBRARY_PREFIX}${RUST_LIB_NAME}${CMAKE_STATIC_LIBRARY_SUFFIX}
    dl
    rt
)

# Include directories
target_include_directories(unifiedshield
    PRIVATE
    ${CMAKE_SOURCE_DIR}/src
    ${CMAKE_SOURCE_DIR}/../core/include
)

# Compiler flags
target_compile_options(unifiedshield PRIVATE
    -Wall -Wextra -Werror -O2
    -fstack-protector-strong
    -D_FORTIFY_SOURCE=2
)

# Linker flags
target_link_options(unifiedshield PRIVATE
    -Wl,-z,relro
    -Wl,-z,now
    -Wl,--as-needed
)

# Install targets
install(TARGETS unifiedshield RUNTIME DESTINATION bin)

# Install configuration files
install(FILES ${CMAKE_SOURCE_DIR}/unifiedshield.service DESTINATION ${CMAKE_INSTALL_LIBDIR}/systemd/system)
install(DIRECTORY DESTINATION /etc/unifiedshield)

# Set permissions
install(CODE "execute_process(COMMAND chmod 600 /etc/unifiedshield/config.json)")

# Packaging
set(CPACK_GENERATOR "DEB;RPM;TGZ")
set(CPACK_PACKAGE_NAME "unifiedshield")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Next-gen anti-censorship VPN for Iran")
set(CPACK_PACKAGE_LICENSE "GPL-3.0")
set(CPACK_PACKAGE_CONTACT "UnifiedShield <team@unifiedshield.org>")

# Debian specific
set(CPACK_DEBIAN_PACKAGE_DEPENDS "libc6 (>= 2.31), libgcc-s1 (>= 10)")
set(CPACK_DEBIAN_PACKAGE_SECTION "net")
set(CPACK_DEBIAN_PACKAGE_PRIORITY "optional")
set(CPACK_DEBIAN_PACKAGE_MAINTAINER "UnifiedShield <team@unifiedshield.org>")

# RPM specific
set(CPACK_RPM_PACKAGE_LICENSE "GPL-3.0")
set(CPACK_RPM_PACKAGE_GROUP "Applications/Internet")

include(CPack)
