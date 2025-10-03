
#ifndef TRUVIXX_INTERFACE_API_H
#define TRUVIXX_INTERFACE_API_H

#ifdef TRUVIXX_INTERFACE_STATIC_DEFINE
#  define TRUVIXX_INTERFACE_API
#  define TRUVIXX_INTERFACE_NO_EXPORT
#else
#  ifndef TRUVIXX_INTERFACE_API
#    ifdef truvixx_interface_EXPORTS
        /* We are building this library */
#      define TRUVIXX_INTERFACE_API __declspec(dllexport)
#    else
        /* We are using this library */
#      define TRUVIXX_INTERFACE_API __declspec(dllimport)
#    endif
#  endif

#  ifndef TRUVIXX_INTERFACE_NO_EXPORT
#    define TRUVIXX_INTERFACE_NO_EXPORT 
#  endif
#endif

#ifndef TRUVIXX_INTERFACE_DEPRECATED
#  define TRUVIXX_INTERFACE_DEPRECATED __declspec(deprecated)
#endif

#ifndef TRUVIXX_INTERFACE_DEPRECATED_EXPORT
#  define TRUVIXX_INTERFACE_DEPRECATED_EXPORT TRUVIXX_INTERFACE_API TRUVIXX_INTERFACE_DEPRECATED
#endif

#ifndef TRUVIXX_INTERFACE_DEPRECATED_NO_EXPORT
#  define TRUVIXX_INTERFACE_DEPRECATED_NO_EXPORT TRUVIXX_INTERFACE_NO_EXPORT TRUVIXX_INTERFACE_DEPRECATED
#endif

/* NOLINTNEXTLINE(readability-avoid-unconditional-preprocessor-if) */
#if 0 /* DEFINE_NO_DEPRECATED */
#  ifndef TRUVIXX_INTERFACE_NO_DEPRECATED
#    define TRUVIXX_INTERFACE_NO_DEPRECATED
#  endif
#endif

#endif /* TRUVIXX_INTERFACE_API_H */
