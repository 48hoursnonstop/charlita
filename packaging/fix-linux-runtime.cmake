# Non-Qt dependencies from the build distribution also need to find their
# bundled dependencies. RUNPATH on the executable does not cover grandchildren.
find_program(PATCHELF_EXECUTABLE patchelf REQUIRED)
file(GLOB libraries "${CMAKE_INSTALL_PREFIX}/lib/*.so*")
foreach(library IN LISTS libraries)
    if(NOT IS_SYMLINK "${library}")
        execute_process(COMMAND "${PATCHELF_EXECUTABLE}" --set-rpath "$ORIGIN" "${library}"
            COMMAND_ERROR_IS_FATAL ANY)
    endif()
endforeach()
