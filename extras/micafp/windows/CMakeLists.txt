cmake_minimum_required(VERSION 3.22)
project(UnifiedShield VERSION 1.0.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Windows-specific settings
if(WIN32)
    add_definitions(-DUNICODE -D_UNICODE)
    add_definitions(-DWIN32_LEAN_AND_MEAN)
    add_definitions(-DNOMINMAX)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
endif()

# Find Rust static library (built by cargo)
set(RUST_LIB_DIR "${CMAKE_SOURCE_DIR}/../target/release")
set(RUST_LIB_NAME "unifiedshield")

# Wintun dependency
set(WINTUN_DIR "${CMAKE_SOURCE_DIR}/deps/wintun")
if(EXISTS "${WINTUN_DIR}/include")
    include_directories(${WINTUN_DIR}/include)
    link_directories(${WINTUN_DIR}/bin/amd64)
endif()

# Source files
set(SOURCES
    src/main.cpp
    src/wintun_interface.cpp
    src/service_manager.cpp
    src/tray_icon.cpp
    src/ipc_client.cpp
)

# Resource files
set(RESOURCES
    src/resource.rc
)

# Create Windows executable
add_executable(unifiedshield WIN32 ${SOURCES} ${RESOURCES})

# Link libraries
target_link_libraries(unifiedshield
    PRIVATE
    comctl32
    shell32
    advapi32
    kernel32
    user32
    gdi32
    ole32
    uuid
    ws2_32
    crypt32
    bcrypt
    ${RUST_LIB_DIR}/${CMAKE_STATIC_LIBRARY_PREFIX}${RUST_LIB_NAME}${CMAKE_STATIC_LIBRARY_SUFFIX}
)

# Include directories
target_include_directories(unifiedshield
    PRIVATE
    ${CMAKE_SOURCE_DIR}/src
    ${CMAKE_SOURCE_DIR}/../core/include
)

# Compiler warnings
if(MSVC)
    target_compile_options(unifiedshield PRIVATE /W4 /WX /O2)
    target_compile_definitions(unifiedshield PRIVATE
        _CRT_SECURE_NO_WARNINGS
        WINVER=0x0A00
        _WIN32_WINNT=0x0A00
    )
endif()

# Copy Wintun DLL to output directory
if(EXISTS "${WINTUN_DIR}/bin/amd64/wintun.dll")
    add_custom_command(TARGET unifiedshield POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${WINTUN_DIR}/bin/amd64/wintun.dll"
            $<TARGET_FILE_DIR:unifiedshield>
    )
endif()

# Install targets
install(TARGETS unifiedshield RUNTIME DESTINATION .)
install(FILES "${WINTUN_DIR}/bin/amd64/wintun.dll" DESTINATION . OPTIONAL)
