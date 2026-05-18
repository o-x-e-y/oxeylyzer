{ lib, stdenv, fetchurl, dpkg, autoPatchelfHook, wrapGAppsHook4, webkitgtk_4_1, gtk3, librsvg }:

stdenv.mkDerivation rec {
  pname = "oxeylyzer";
  version = "0.1.0";

  src = fetchurl {
    url = "https://github.com/o-x-e-y/oxeylyzer/releases/download/oxeylyzer-v${version}/oxeylyzer_${version}_amd64.deb";
    hash = "sha256-UA2OL0MsFFy2yOWhGeUTSCYpveVI/JufqAwOBYbHnk0=";
  };

  nativeBuildInputs = [
    dpkg
    autoPatchelfHook
    wrapGAppsHook4
  ];

  buildInputs = [
    webkitgtk_4_1
    gtk3
    librsvg
  ];

  dontBuild = true;
  dontStrip = true;

  unpackPhase = "dpkg-deb -x $src .";

  installPhase = ''
    mkdir -p $out/bin $out/share/applications $out/share/icons
    cp usr/bin/oxeylyzer-tauri $out/bin/oxeylyzer
    cp usr/share/applications/oxeylyzer.desktop $out/share/applications/
    cp -r usr/share/icons $out/share/
  '';

  meta = with lib; {
    description = "A web frontend for Oxeylyzer, a keyboard layout analyzer";
    homepage = "https://github.com/o-x-e-y/oxeylyzer";
    license = licenses.asl20;
    platforms = [ "x86_64-linux" ];
    mainProgram = "oxeylyzer";
  };
}
