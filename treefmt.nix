{
  projectRootFile = "flake.nix";
  programs.rustfmt.enable = true;
  programs.prettier.enable = true;
  programs.shfmt.enable = true;
  settings.formatter.prettier.excludes = [
    "static/lib/**"
  ];
}
