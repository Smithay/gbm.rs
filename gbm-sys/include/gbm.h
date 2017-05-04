/*
 * Copyright © 2011 Intel Corporation
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice (including the next
 * paragraph) shall be included in all copies or substantial portions of the
 * Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT.  IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 * HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
 * WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 *
 * Authors:
 *    Benjamin Franzke <benjaminfranzke@googlemail.com>
 */

#ifndef _GBM_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif


/**
 * \file gbm.h
 * \brief Generic Buffer Manager
 */

struct gbm_device;
struct gbm_bo;
struct gbm_surface;

/**
 * \mainpage The Generic Buffer Manager
 *
 * This module provides an abstraction that the caller can use to request a
 * buffer from the underlying memory management system for the platform.
 *
 * This allows the creation of portable code whilst still allowing access to
 * the underlying memory manager.
 */

/**
 * Abstraction representing the handle to a buffer allocated by the
 * manager
 */
union gbm_bo_handle {
   void *ptr;
   int32_t s32;
   uint32_t u32;
   int64_t s64;
   uint64_t u64;
};

/** Format of the allocated buffer */
enum gbm_bo_format {
   /** RGB with 8 bits per channel in a 32 bit value */
   GBM_BO_FORMAT_XRGB8888,
   /** ARGB with 8 bits per channel in a 32 bit value */
   GBM_BO_FORMAT_ARGB8888
};

/**
 * Flags to indicate the intended use for the buffer - these are passed into
 * gbm_bo_create(). The caller must set the union of all the flags that are
 * appropriate
 *
 * \sa Use gbm_device_is_format_supported() to check if the combination of format
 * and use flags are supported
 */
enum gbm_bo_flags {
   /**
    * Buffer is going to be presented to the screen using an API such as KMS
    */
   GBM_BO_USE_SCANOUT      = (1 << 0),
   /**
    * Buffer is going to be used as cursor
    */
   GBM_BO_USE_CURSOR       = (1 << 1),
   /**
    * Deprecated
    */
   GBM_BO_USE_CURSOR_64X64 = GBM_BO_USE_CURSOR,
   /**
    * Buffer is to be used for rendering - for example it is going to be used
    * as the storage for a color buffer
    */
   GBM_BO_USE_RENDERING    = (1 << 2),
   /**
    * Buffer can be used for gbm_bo_write.  This is guaranteed to work
    * with GBM_BO_USE_CURSOR, but may not work for other combinations.
    */
   GBM_BO_USE_WRITE    = (1 << 3),
   /**
    * Buffer is linear, i.e. not tiled.
    */
   GBM_BO_USE_LINEAR = (1 << 4),
};

int
gbm_device_get_fd(struct gbm_device *gbm);

const char *
gbm_device_get_backend_name(struct gbm_device *gbm);

int
gbm_device_is_format_supported(struct gbm_device *gbm,
                               uint32_t format, uint32_t usage);

void
gbm_device_destroy(struct gbm_device *gbm);

struct gbm_device *
gbm_create_device(int fd);

struct gbm_bo *
gbm_bo_create(struct gbm_device *gbm,
              uint32_t width, uint32_t height,
              uint32_t format, uint32_t flags);

#define GBM_BO_IMPORT_WL_BUFFER         0x5501
#define GBM_BO_IMPORT_EGL_IMAGE         0x5502
#define GBM_BO_IMPORT_FD                0x5503

struct gbm_import_fd_data {
   int fd;
   uint32_t width;
   uint32_t height;
   uint32_t stride;
   uint32_t format;
};

struct gbm_bo *
gbm_bo_import(struct gbm_device *gbm, uint32_t type,
              void *buffer, uint32_t usage);

/**
 * Flags to indicate the type of mapping for the buffer - these are
 * passed into gbm_bo_map(). The caller must set the union of all the
 * flags that are appropriate.
 *
 * These flags are independent of the GBM_BO_USE_* creation flags. However,
 * mapping the buffer may require copying to/from a staging buffer.
 *
 * See also: pipe_transfer_usage
 */
enum gbm_bo_transfer_flags {
   /**
    * Buffer contents read back (or accessed directly) at transfer
    * create time.
    */
   GBM_BO_TRANSFER_READ       = (1 << 0),
   /**
    * Buffer contents will be written back at unmap time
    * (or modified as a result of being accessed directly).
    */
   GBM_BO_TRANSFER_WRITE      = (1 << 1),
   /**
    * Read/modify/write
    */
   GBM_BO_TRANSFER_READ_WRITE = (GBM_BO_TRANSFER_READ | GBM_BO_TRANSFER_WRITE),
};

void *
gbm_bo_map(struct gbm_bo *bo,
           uint32_t x, uint32_t y, uint32_t width, uint32_t height,
           uint32_t flags, uint32_t *stride, void **map_data);

void
gbm_bo_unmap(struct gbm_bo *bo, void *map_data);

uint32_t
gbm_bo_get_width(struct gbm_bo *bo);

uint32_t
gbm_bo_get_height(struct gbm_bo *bo);

uint32_t
gbm_bo_get_stride(struct gbm_bo *bo);

uint32_t
gbm_bo_get_format(struct gbm_bo *bo);

struct gbm_device *
gbm_bo_get_device(struct gbm_bo *bo);

union gbm_bo_handle
gbm_bo_get_handle(struct gbm_bo *bo);

int
gbm_bo_get_fd(struct gbm_bo *bo);

int
gbm_bo_write(struct gbm_bo *bo, const void *buf, size_t count);

void
gbm_bo_set_user_data(struct gbm_bo *bo, void *data,
		     void (*destroy_user_data)(struct gbm_bo *, void *));

void *
gbm_bo_get_user_data(struct gbm_bo *bo);

void
gbm_bo_destroy(struct gbm_bo *bo);

struct gbm_surface *
gbm_surface_create(struct gbm_device *gbm,
                   uint32_t width, uint32_t height,
		   uint32_t format, uint32_t flags);

int
gbm_surface_needs_lock_front_buffer(struct gbm_surface *surface);

struct gbm_bo *
gbm_surface_lock_front_buffer(struct gbm_surface *surface);

void
gbm_surface_release_buffer(struct gbm_surface *surface, struct gbm_bo *bo);

int
gbm_surface_has_free_buffers(struct gbm_surface *surface);

void
gbm_surface_destroy(struct gbm_surface *surface);

#ifdef __cplusplus
}
#endif

#endif
