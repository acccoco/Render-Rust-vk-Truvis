
#ifndef TRUVIXX_ASSIMP_API_H
#define TRUVIXX_ASSIMP_API_H

#ifdef TRUVIXX_ASSIMP_STATIC_DEFINE
#  define TRUVIXX_ASSIMP_API
#  define TRUVIXX_ASSIMP_NO_EXPORT
#else
#  ifndef TRUVIXX_ASSIMP_API
#    ifdef truvixx_assimp_EXPORTS
        /* We are building this library */
#      define TRUVIXX_ASSIMP_API __declspec(dllexport)
#    else
        /* We are using this library */
#      define TRUVIXX_ASSIMP_API __declspec(dllimport)
#    endif
#  endif

#  ifndef TRUVIXX_ASSIMP_NO_EXPORT
#    define TRUVIXX_ASSIMP_NO_EXPORT 
#  endif
#endif

#ifndef TRUVIXX_ASSIMP_DEPRECATED
#  define TRUVIXX_ASSIMP_DEPRECATED __declspec(deprecated)
#endif

#ifndef TRUVIXX_ASSIMP_DEPRECATED_EXPORT
#  define TRUVIXX_ASSIMP_DEPRECATED_EXPORT TRUVIXX_ASSIMP_API TRUVIXX_ASSIMP_DEPRECATED
#endif

#ifndef TRUVIXX_ASSIMP_DEPRECATED_NO_EXPORT
#  define TRUVIXX_ASSIMP_DEPRECATED_NO_EXPORT TRUVIXX_ASSIMP_NO_EXPORT TRUVIXX_ASSIMP_DEPRECATED
#endif

/* NOLINTNEXTLINE(readability-avoid-unconditional-preprocessor-if) */
#if 0 /* DEFINE_NO_DEPRECATED */
#  ifndef TRUVIXX_ASSIMP_NO_DEPRECATED
#    define TRUVIXX_ASSIMP_NO_DEPRECATED
#  endif
#endif

#endif /* TRUVIXX_ASSIMP_API_H */
