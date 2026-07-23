{ pkgs, lib, ... }:

{
  dotenv.enable = true;

  dagger.enable = true;
  env.DAGGER_X_RELEASE = "v1.0.0-beta.7";

  # Required by arborium
  env.CC_wasm32_unknown_unknown = "${pkgs.llvmPackages.clang-unwrapped}/bin/clang";

  packages =
    with pkgs;
    [
      lld
      cargo-audit
      cargo-deny
      cargo-dist
      cargo-release
      cargo-watch
      wasm-pack
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
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
    };
  };
}
