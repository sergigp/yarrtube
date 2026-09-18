<h1>
<p align="center">
  <img src="doc/logo.png" alt="Logo" width="256">
</h1>
  <p align="center">
    Minimalistic YouTube synchronizer built on top of <a href="https://github.com/yt-dlp/yt-dlp">yt-dlp</a>
    <br />
    <a href="#about">About</a>
    ·
    <a href="#motivation">Motivation</a>
    ·
    <a href="doc/INSTALLATION.md">Installation</a>
    ·
    <a href="doc/DEVELOPMENT.md">Developing</a>
    ·
    <a href="doc/ARCHITECTURE.md">Architecture</a>
    ·
    <a href="doc/ARCHITECTURE.md">License</a>
    <br />
    <img src="doc/screenshot.jpeg" alt="Yarrtube web UI screenshot" width="600"/>
  </p>
</p>

# About

Yarrtube watches tracked Youtube playlists and channels and automatically downloads the new videos published.

Yarrtube is mainly thought to be installed on your NAS via Docker with the rest of your media stack (Plex, Jellyfin, etc.) but it can also be installed on any laptop.

# Motivation

This is a personal project. I'm a software engineer coming back from a ~10-month sabbatical, and I wanted a real, finished thing to get my hands dirty again, both with writing code and with AI-assisted coding.

It also solves an actual problem at home: I wanted a reliable, ad-free way to watch specific channels and playlists in Plex, especially for my kids. Yarrtube keeps the playlists and channels I care about mirrored to disk with media-server-friendly naming, so they just show up in Plex — no ads, no algorithm, no feed.

It doesn't aim to compete with anything. If you want a full-featured archiver, projects like TubeArchivist do far more. Yarrtube is small on purpose.

# Main Features

- Track Youtube public playlists.
- Track entire channels and download new videos when they are published!
- Minimalistic webapp to see the videos from the browser in both desktop and mobile.
- Basic support for Plex, Kodi and Jellyfin.
- **Coming soon** custom playlists that let you create playlists not coupled to Youtube ones.
- **Coming soon** browser extensions to add videos to custom playlists directly from Youtube
- **Coming soon** extended support for Plex Collections.
