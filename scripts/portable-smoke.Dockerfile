FROM ubuntu:22.04
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    python3 python3-pil xvfb xauth xdotool dbus-x11 libgl1 libegl1 libgles2 libopengl0 ca-certificates \
    xdg-utils libglib2.0-bin && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 tester
# Deliberately no system GTK, WebKitGTK, Wine, or FUSE installation.
USER tester
ENV APPIMAGE_EXTRACT_AND_RUN=1
WORKDIR /source
