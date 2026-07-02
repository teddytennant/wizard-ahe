{
  description = "Nix-packaged base image for wizard AHE task environments";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # python3 with pytest importable (`python3 -m pytest` and `pytest` both work).
      python = pkgs.python312.withPackages (ps: [ ps.pytest ]);
    in
    {
      packages.${system} = {
        taskImage = pkgs.dockerTools.streamLayeredImage {
          name = "wizard-ahe/task-base";
          tag = "latest";

          contents = [
            # /bin/sh, /usr/bin/env, /etc/passwd & friends
            pkgs.dockerTools.binSh
            pkgs.dockerTools.usrBinEnv
            pkgs.dockerTools.fakeNss

            pkgs.bash
            pkgs.coreutils
            pkgs.gnugrep
            pkgs.gnused
            pkgs.gawk
            pkgs.findutils
            pkgs.diffutils
            pkgs.gnumake
            pkgs.gnutar
            pkgs.gzip
            pkgs.git
            python
            pkgs.cacert
          ];

          # Runs in the image root while building the customisation layer.
          # /usr/local/bin must exist: harbor `docker compose cp`s the
          # agent binary to /usr/local/bin/wizard and cp fails if the
          # parent directory is missing.
          extraCommands = ''
            mkdir -p tmp app logs usr/local/bin root
            chmod 1777 tmp
          '';

          config = {
            Env = [
              "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
              "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
              # fakeNss gives root a /var/empty home; agents expect ~/.wizard
              # etc. under /root (matching Debian-based images).
              "HOME=/root"
            ];
            WorkingDir = "/app";
            Cmd = [ "/bin/bash" ];
          };
        };

        default = self.packages.${system}.taskImage;
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = [ pkgs.uv pkgs.python312 ];
      };
    };
}
