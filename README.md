<h1>
<p align="center">
  <img src="doc/logo.png" alt="Logo" width="256">
</h1>
  <p align="center">
    Minimalistic and lightweight self-hosted private YouTube synchronizer built on top of <a href="https://github.com/yt-dlp/yt-dlp">yt-dlp</a>
    <br />
    <a href="#about">About</a>
    ·
    <a href="#motivation">Motivation</a>
    ·
    <a href="doc/INSTALLATION.md">Installation</a>
    ·
    <a href="doc/ARCHITECTURE.md">Architecture</a>
    ·
    <a href="LICENSE">License</a>
    <br />
    <img src="doc/screenshot.jpeg" alt="Yarrtube web UI screenshot" width="600"/>
  </p>
</p>

# About

Yarrtube is a self-hosted service that watches tracked YouTube playlists and channels and automatically downloads the new videos published.

Yarrtube is mainly thought to be installed on your NAS via Docker with the rest of your media stack (Plex, Jellyfin, etc.) but it can also be installed on any laptop.

# Main Features

- Track YouTube public playlists.
- Track entire channels and download new videos when they are published!
- Minimalistic webapp to see the videos from the browser in both desktop and mobile.
- Basic support for Plex, Kodi and Jellyfin.
- Support for Plex collections: one collection per tracked playlist/channel, kept in sync automatically. Take a look at the [Plex Collections integration guide](doc/PLEX.md) for more details.

<p align="center">
  <img src="doc/screenshot_mobile.png" alt="Yarrtube mobile UI screenshot" width="150"/>
  <img src="doc/screenshot_plex.jpeg" alt="Yarrtube Plex Integration screenshot" width="520"/>
</p>

# Motivation

This is a personal project. I'm an experienced software engineer coming back from a ~10-month sabbatical, and I wanted a real project to get my hands dirty again, both with writing code and with AI-assisted coding. I wanted to build something that I would actually use, and that would be useful to others too. Yarrtube is the result. If you are a recruiter/dev you can take a look at the [architecture](doc/ARCHITECTURE.md).

It also solves an actual problem at home: I wanted a reliable, ad-free and no-algorithm way to watch specific channels and playlists in Plex, especially for my kids. Yarrtube keeps the playlists and channels I care about mirrored to disk, so they just show up in Plex. **No ads and no algorithm**.

It doesn't aim to compete with anything. If you want a full-featured archiver, projects like [TubeArchivist](https://www.tubearchivist.com/) do far more. Yarrtube is small on purpose.
