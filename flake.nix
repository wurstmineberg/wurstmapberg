{
    inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/*.tar.gz";
    outputs = attrs: let
        supportedSystems = [
            "aarch64-darwin"
            "aarch64-linux"
            "x86_64-darwin"
            "x86_64-linux"
        ];
        forEachSupportedSystem = f: attrs.nixpkgs.lib.genAttrs supportedSystems (system: f {
            pkgs = import attrs.nixpkgs {
                inherit system;
            };
        });
    in {
        devShells = forEachSupportedSystem ({ pkgs, ... }: {
            default = pkgs.mkShell {
                packages = with pkgs; [
                    cargo
                ];
            };
            flamegraph = pkgs.mkShell {
                packages = with pkgs; [
                    cargo
                    cargo-flamegraph
                ];
                shellHook = ''
                    while [[ $(rsync --delete -ai wurstmineberg@wurstmineberg.de:/opt/wurstmineberg/world/wurstmineberg/ world/) ]]; do
                        echo 'rsync exited with output, repeating'
                    done
                    cargo flamegraph --package=wurstmapberg-cli --features=flamegraph --profile=flamegraph -- world/world
                    exit
                '';
            };
        });
    };
}
