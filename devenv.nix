{ pkgs, lib, ... }:

{
  dotenv.enable = true;

  dagger.enable = true;
  env.DAGGER_X_RELEASE = "86d1d2f5791bcf3213d56903cfa81a3ba0abe54a";

  env.CC_wasm32_unknown_unknown = "${pkgs.llvmPackages.clang-unwrapped}/bin/clang";

  packages = [
    pkgs.lld
    pkgs.wasm-pack
  ]
  ++ lib.optionals pkgs.stdenv.isLinux [
    # Needed by `wasm-pack test --chrome --headless`.
    pkgs.chromium
    pkgs.chromedriver
  ];

  languages = {
    rust = {
      enable = true;
      channel = "stable";
      targets = [ "wasm32-unknown-unknown" ];
    };
    javascript = {
      enable = true;
      npm.enable = true;
      bun.enable = true;
    };
  };
}
