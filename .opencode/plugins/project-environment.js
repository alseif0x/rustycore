// OpenCode V2 environment for this checkout (adapted to 3.4.3 from forever b8330b1b5).
// Explicit caller values win.
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const localWork = join(homedir(), ".local/share/rustycore-opencode-work");
// 3.4.3 has no .protoc-version pin: use the operator's installed protoc.
const defaultProtoc = join(homedir(), ".local/protoc/bin/protoc");
// Full C++ 3.4.3 reference on this server.
const cppReference = "/home/server/woltk-trinity-legacy";

export default {
  id: "rustycore.project-environment",
  async setup(ctx) {
    await ctx.shell.hook("create.before", (event) => {
      const env = event.env;
      const cargoBin = join(homedir(), ".cargo/bin");
      const path = env.PATH ?? process.env.PATH ?? "/usr/bin:/bin";
      env.PATH = [cargoBin, ...path.split(":").filter((p) => p && p !== cargoBin)].join(":");
      env.CARGO_TARGET_DIR ??= join(projectRoot, "target");
      env.CARGO_BUILD_JOBS ??= "1";
      env.VALIDATION_V2_CARGO_JOBS ??= "1";
      env.TMPDIR ??= join(localWork, "tmp");
      env.PYTHONPYCACHEPREFIX ??= join(localWork, "cache/python");
      for (const path of [env.CARGO_TARGET_DIR, env.TMPDIR, env.PYTHONPYCACHEPREFIX]) {
        mkdirSync(path, { recursive: true, mode: 0o700 });
      }
      const pinFile = join(projectRoot, ".protoc-version");
      if (existsSync(pinFile)) {
        const pin = readFileSync(pinFile, "utf8").trim();
        const pinned = join(localWork, `protoc-${pin}/bin/protoc`);
        if (existsSync(pinned)) env.PROTOC ??= pinned;
      }
      if (existsSync(defaultProtoc)) env.PROTOC ??= defaultProtoc;
      if (existsSync(cppReference)) env.RUSTYCORE_CPP_REFERENCE_ROOT ??= cppReference;
      const openssl = join(localWork, "openssl-3.5.5");
      if (existsSync(join(openssl, "include/openssl/ssl.h")) && existsSync(join(openssl, "lib/libssl.so"))) {
        env.OPENSSL_INCLUDE_DIR ??= join(openssl, "include");
        env.OPENSSL_LIB_DIR ??= join(openssl, "lib");
      }
    });
  },
};
