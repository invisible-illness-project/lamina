# The Flutter tooling requires that developers have CMake 3.10 or later
# installed. You should not increase this version, as doing so will cause
# the plugin to fail to compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.10)

project(lamina_dart_library VERSION 0.0.1 LANGUAGES C)

add_library(lamina_dart SHARED
  "lamina_dart.c"
)

set_target_properties(lamina_dart PROPERTIES
  PUBLIC_HEADER lamina_dart.h
  OUTPUT_NAME "lamina_dart"
)

target_compile_definitions(lamina_dart PUBLIC DART_SHARED_LIB)

if (ANDROID)
  # Support Android 15 16k page size
  target_link_options(lamina_dart PRIVATE "-Wl,-z,max-page-size=16384")
endif()
