self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.jmapper;
  settingsFormat = pkgs.formats.toml { };
  # Defaults injected under `settings` so a minimal `enable = true` just works;
  # anything set in `cfg.settings` wins via recursiveUpdate. `database_url` is
  # only defaulted when this module provisions the local DB (createDatabase),
  # where the socket path and db name (= the service user) are known.
  defaultSettings = {
    server = {
      bind = "127.0.0.1:8765";
    }
    // lib.optionalAttrs cfg.createDatabase {
      database_url = "host=/run/postgresql dbname=${cfg.user}";
    };
  };
  mergedSettings = lib.recursiveUpdate defaultSettings cfg.settings;
  baseFile = settingsFormat.generate "jmapper-base.toml" mergedSettings;
in
{
  options.services.jmapper = {
    enable = lib.mkEnableOption "JMAP-over-IMAP bridge";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.jmapper;
      defaultText = lib.literalExpression "jmapper.packages.\${system}.jmapper";
      description = "The jmapper package to run.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "jmapper";
      description = "User jmapper runs as.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "jmapper";
      description = "Primary group for the jmapper user.";
    };

    settings = lib.mkOption {
      inherit (settingsFormat) type;
      default = { };
      description = ''
        Non-secret portion of the TOML config, rendered into the Nix store.
        Do NOT put bearer tokens or mail credentials here, use
        `accountsFile` instead, which is concatenated at activation time and
        stays out of the store.

        `server.bind` defaults to `127.0.0.1:8765` and, when `createDatabase`
        is on, `server.database_url` defaults to the provisioned local socket
        (`host=/run/postgresql dbname=<user>`); set either here to override.
        `server.session_url` and `server.cors_origins` are site-specific and
        have no default.
      '';
      example = lib.literalExpression ''
        {
          server = {
            bind = "127.0.0.1:8765";
            session_url = "https://mail.example.com";
            cors_origins = [ "https://webmail.example.com" ];
            database_url = "host=/run/postgresql";
          };
        }
      '';
    };

    accountsFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Path (outside the Nix store) to a TOML fragment containing
        `[[accounts]]` entries and their Gmail, IMAP, SMTP, CalDAV, and
        CardDAV sub-tables. Concatenated onto the base settings file at service start.
        Typical layout: managed by agenix / sops-nix, owned by the jmapper user,
        mode 0400.
      '';
      example = "/run/secrets/jmapper-accounts.toml";
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Optional EnvironmentFile for the systemd service (e.g. to inject
        `JMAPPER_LOG=debug`). Not required.
      '';
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Open the `server.bind` TCP port in the firewall. Leave false if jmapper
        sits behind nginx on localhost (the common case).
      '';
    };

    createDatabase = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Provision a local PostgreSQL database + role (peer-authenticated over
        the unix socket) and order jmapper after postgresql.service. Disable
        when pointing `settings.server.database_url` at an external server.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    services.postgresql = lib.mkIf cfg.createDatabase {
      enable = true;
      ensureDatabases = [ cfg.user ];
      ensureUsers = [
        {
          name = cfg.user;
          ensureDBOwnership = true;
        }
      ];
    };

    users.users.${cfg.user} = {
      isSystemUser = true;
      inherit (cfg) group;
      home = "/var/lib/${cfg.user}";
      description = "jmapper service user";
    };
    users.groups.${cfg.group} = { };

    systemd.tmpfiles.rules = [
      "d /var/lib/${cfg.user} 0750 ${cfg.user} ${cfg.group} -"
    ];

    systemd.services.jmapper = {
      description = "JMAP-over-IMAP bridge";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ] ++ lib.optional cfg.createDatabase "postgresql.service";
      requires = lib.optional cfg.createDatabase "postgresql.service";
      wants = [ "network-online.target" ];

      # Preamble concatenates the non-secret base file with the secret
      # accounts fragment into a root-readable runtime file.
      preStart =
        let
          destination = "/run/jmapper/jmapper.toml";
        in
        ''
          {
            cat ${baseFile}
            ${lib.optionalString (cfg.accountsFile != null) ''
              printf '\n\n'
              cat ${lib.escapeShellArg (toString cfg.accountsFile)}
            ''}
          } > ${destination}
        '';

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/jmapper --config /run/jmapper/jmapper.toml run";
        ExecReload = "${pkgs.coreutils}/bin/kill -HUP $MAINPID";
        User = cfg.user;
        Group = cfg.group;
        Restart = "on-failure";
        RestartSec = 5;
        TimeoutStopSec = 30;
        StateDirectory = cfg.user;
        StateDirectoryMode = "0750";
        RuntimeDirectory = "jmapper";
        RuntimeDirectoryMode = "0750";
        WorkingDirectory = "/var/lib/${cfg.user}";

        # Standard hardening for the generated systemd unit.
        ProtectSystem = "strict";
        ReadWritePaths = [ "/var/lib/${cfg.user}" ];
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        PrivateUsers = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHostname = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged @resources"
        ];
        CapabilityBoundingSet = "";
        AmbientCapabilities = "";
        NoNewPrivileges = true;
        UMask = "0027";
        LimitNOFILE = 4096;
        TasksMax = 64;
        MemoryHigh = "256M";
        MemoryMax = "512M";

        EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
      };

      environment = {
        JMAPPER_LOG = "info,jmapper=info,imap_sync=info,dav_sync=info,jmap_server=info";
      };
    };

    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts =
        let
          port = lib.toInt (lib.last (lib.splitString ":" mergedSettings.server.bind));
        in
        [ port ];
    };
  };
}
