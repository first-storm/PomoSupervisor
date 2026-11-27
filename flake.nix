{
  description = "PomoSupervisor: a menu-bar pomodoro timer that says when you drift off the task";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      # macOS only: the app is built on the Accessibility, CoreGraphics and AppKit APIs.
      systems = [ "aarch64-darwin" "x86_64-darwin" ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forEachSystem (pkgs: {
        # NoCC on purpose: the app must link with the system clang against the installed macOS SDK.
        # nixpkgs' own SDK is older, and AppKit decides how the app looks from the SDK it was linked with.
        default = pkgs.mkShellNoCC {
          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            nodejs
            pnpm
            # icon/README.md: rsvg-convert renders the flat and menu-bar icons.
            librsvg
          ];
          # Leave the deployment target to cargo and tauri (bundle.macOS.minimumSystemVersion), as outside the shell.
          shellHook = "unset MACOSX_DEPLOYMENT_TARGET";
        };
      });
    };
}
