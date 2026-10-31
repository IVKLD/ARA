{
  description = "ARA Retained UI Engine (Pure Wayland + Vulkan)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      libPath = pkgs.lib.makeLibraryPath [
        pkgs.wayland
        pkgs.libxkbcommon
        pkgs.vulkan-loader
        pkgs.libGL
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          rust-analyzer
          pkg-config
        ];

        buildInputs = with pkgs; [
          wayland
          wayland-protocols
          libxkbcommon
          vulkan-loader
          vulkan-headers
          libGL
        ];

        shellHook = ''
          export WAYLAND_DISPLAY="''${WAYLAND_DISPLAY:-wayland-0}"
          export LD_LIBRARY_PATH="${libPath}:''${LD_LIBRARY_PATH:-}"
        '';
      };
    };
}
