// Isolated harness client: real xdg min/max constraints and delayed commits.
// No compositor equations are copied. Refuses to start outside the harness.
#define _GNU_SOURCE
#include "xdg-shell-client-protocol.h"
#include <wayland-client.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

struct client {
  struct wl_display *display;
  struct wl_compositor *compositor;
  struct wl_shm *shm;
  struct xdg_wm_base *wm;
  struct wl_surface *surface;
  struct xdg_surface *xdg;
  struct xdg_toplevel *top;
  int width, height, min_w, min_h, max_w, max_h, delay, mapped;
};
static int env_int(const char *name) {
  const char *s = getenv(name);
  return s ? atoi(s) : 0;
}
static int bounded(int size, int min, int max) {
  if (size < min) size = min;
  if (max && size > max) size = max;
  return size;
}
static void ping(void *data, struct xdg_wm_base *wm, uint32_t serial) {
  (void)data;
  xdg_wm_base_pong(wm, serial);
}
static const struct xdg_wm_base_listener wm_listener = {.ping = ping};
static void global(void *data, struct wl_registry *registry, uint32_t id,
                   const char *interface, uint32_t version) {
  (void)version;
  struct client *c = data;
  if (!strcmp(interface, "wl_compositor"))
    c->compositor = wl_registry_bind(registry, id, &wl_compositor_interface, 4);
  else if (!strcmp(interface, "wl_shm"))
    c->shm = wl_registry_bind(registry, id, &wl_shm_interface, 1);
  else if (!strcmp(interface, "xdg_wm_base")) {
    c->wm = wl_registry_bind(registry, id, &xdg_wm_base_interface, 1);
    xdg_wm_base_add_listener(c->wm, &wm_listener, c);
  }
}
static void removed(void *data, struct wl_registry *r, uint32_t id) {
  (void)data; (void)r; (void)id;
}
static const struct wl_registry_listener registry_listener = {global, removed};
static void size(void *data, struct xdg_toplevel *top, int32_t w, int32_t h,
                 struct wl_array *states) {
  (void)top; (void)states;
  struct client *c = data;
  if (w > 0) c->width = bounded(w, c->min_w, c->max_w);
  if (h > 0) c->height = bounded(h, c->min_h, c->max_h);
}
static void closed(void *data, struct xdg_toplevel *top) {
  (void)data; (void)top;
  exit(0);
}
static const struct xdg_toplevel_listener top_listener = {.configure = size, .close = closed};
static void configure(void *data, struct xdg_surface *xdg, uint32_t serial) {
  struct client *c = data;
  if (c->mapped && c->delay) usleep((useconds_t)c->delay * 1000);
  xdg_surface_ack_configure(xdg, serial);
  const size_t bytes = (size_t)c->width * c->height * 4;
  int fd = memfd_create("zk-edge-client", MFD_CLOEXEC);
  if (fd < 0 || ftruncate(fd, (off_t)bytes)) exit(2);
  uint32_t *pixels = mmap(NULL, bytes, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
  if (pixels == MAP_FAILED) exit(2);
  for (size_t i = 0; i < bytes / 4; ++i) pixels[i] = 0xff336699;
  struct wl_shm_pool *pool = wl_shm_create_pool(c->shm, fd, (int)bytes);
  struct wl_buffer *buffer = wl_shm_pool_create_buffer(pool, 0, c->width, c->height,
                                                      c->width * 4, WL_SHM_FORMAT_XRGB8888);
  wl_surface_attach(c->surface, buffer, 0, 0);
  wl_surface_damage(c->surface, 0, 0, c->width, c->height);
  wl_surface_commit(c->surface);
  wl_buffer_destroy(buffer);
  wl_shm_pool_destroy(pool);
  munmap(pixels, bytes);
  close(fd);
  c->mapped = 1;
  printf("committed %dx%d\n", c->width, c->height);
  fflush(stdout);
}
static const struct xdg_surface_listener surface_listener = {.configure = configure};
int main(int argc, char **argv) {
  if (!getenv("UMBRIEL_HARNESS_LIB") || argc != 2) return 2;
  struct client c = {.width = 480, .height = 200};
  c.min_w = env_int("MIN_W"); c.min_h = env_int("MIN_H");
  c.max_w = env_int("MAX_W"); c.max_h = env_int("MAX_H");
  c.delay = env_int("COMMIT_DELAY_MS");
  c.width = bounded(c.width, c.min_w, c.max_w);
  c.height = bounded(c.height, c.min_h, c.max_h);
  c.display = wl_display_connect(NULL);
  if (!c.display) return 2;
  struct wl_registry *registry = wl_display_get_registry(c.display);
  wl_registry_add_listener(registry, &registry_listener, &c);
  if (wl_display_roundtrip(c.display) < 0 || !c.compositor || !c.shm || !c.wm) return 2;
  c.surface = wl_compositor_create_surface(c.compositor);
  c.xdg = xdg_wm_base_get_xdg_surface(c.wm, c.surface);
  xdg_surface_add_listener(c.xdg, &surface_listener, &c);
  c.top = xdg_surface_get_toplevel(c.xdg);
  xdg_toplevel_add_listener(c.top, &top_listener, &c);
  xdg_toplevel_set_title(c.top, argv[1]);
  xdg_toplevel_set_min_size(c.top, c.min_w, c.min_h);
  xdg_toplevel_set_max_size(c.top, c.max_w, c.max_h);
  wl_surface_commit(c.surface);
  while (wl_display_dispatch(c.display) >= 0) {}
  return 0;
}
