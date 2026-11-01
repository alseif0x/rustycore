// Local OpenCode V2 environment for this checkout. Explicit caller values win.
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const laboratory = resolve(projectRoot, "../rustycore-extractor-work");
const localWork = join(homedir(), ".local/share/rustycore-opencode-work");

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
      const pin = readFileSync(join(projectRoot, ".protoc-version"), "utf8").trim();
      const protoc = join(laboratory, `cache/devdeps/protoc-${pin}/extracted/bin/protoc`);
      if (existsSync(protoc)) env.PROTOC ??= protoc;
      const sqlReference = join(laboratory, "builds/legacy-sql-reference-92796557");
      if (existsSync(join(sqlReference, "src/server/database/Database/Implementation/CharacterDatabase.cpp"))) {
        env.RUSTYCORE_CPP_REFERENCE_ROOT ??= sqlReference;
      }
      const openssl = join(localWork, "openssl-3.5.5");
      if (existsSync(join(openssl, "include/openssl/ssl.h")) && existsSync(join(openssl, "lib/libssl.so"))) {
        env.OPENSSL_INCLUDE_DIR ??= join(openssl, "include");
        env.OPENSSL_LIB_DIR ??= join(openssl, "lib");
      }
    });
  },
};
