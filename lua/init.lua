-- Momarchy Home configuration.
--
-- This file is compiled into the Momarchy binary and written to
-- ~/.config/momarchy/init.lua on first run. After that, the user's copy is
-- authoritative and can be edited live without rebuilding Momarchy.

return {
  version = 1,
  home = "home",

  screens = {
    home = {
      title = "MOMARCHY",
      subtitle = "Mitä haluat tehdä?",

      buttons = {
        {
          id = "internet",
          label = "INTERNET",
          hint = "Avaa selain",
          action = {
            open = "https://www.google.fi/",
            live_message = "Avataan internet.",
          },
        },
        {
          id = "email",
          label = "SÄHKÖPOSTI",
          hint = "Lue ja lähetä viestejä",
          action = {
            message = "Sähköposti otetaan käyttöön seuraavaksi.",
          },
        },
        {
          id = "photos",
          label = "KUVAT",
          hint = "Katso kuvia",
          action = {
            message = "Kuvat otetaan käyttöön seuraavaksi.",
          },
        },
        {
          id = "youtube",
          label = "YOUTUBE",
          hint = "Katso videoita",
          action = {
            open = "https://www.youtube.com/",
            live_message = "Avataan YouTube.",
          },
        },
        {
          id = "ask",
          label = "KYSY MITÄ VAIN",
          hint = "Kirjoita tai puhu kysymys",
          action = {
            message = "Kysy mitä vain tulee seuraavaksi.",
          },
        },
        {
          id = "tv",
          label = "KATSO TELEVISIOSTA",
          hint = "Chromecast",
          action = {
            message = "Chromecast-tuki tulee seuraavaksi.",
          },
        },
        {
          id = "games",
          label = "PELIT",
          hint = "Palikat, Mato...",
          action = {
            screen = "games",
          },
        },
        {
          id = "help",
          label = "APUA",
          hint = "Jos jokin ei toimi",
          action = {
            screen = "help",
          },
        },
      },
    },

    games = {
      title = "PELIT",
      subtitle = "Valitse peli",

      buttons = {
        {
          id = "palikat",
          label = "PALIKAT",
          hint = "Putoavia palikoita",
          action = {
            message = "Palikat tulee pian :)",
          },
        },
        {
          id = "mato",
          label = "MATO",
          hint = "Syö ja kasva",
          action = {
            message = "Mato tulee pian :)",
          },
        },
        {
          id = "back",
          label = "TAKAISIN",
          hint = "Palaa alkuun",
          action = {
            screen = "home",
          },
        },
      },
    },

    help = {
      title = "APUA",
      subtitle = "Jos jokin ei toimi",
      body = "Momarchy tarkistaa myöhemmin tästä internet-yhteyden ja muut tärkeät asiat.\n\nJos jokin ei toimi, pyydä apua.",

      buttons = {
        {
          id = "back",
          label = "TAKAISIN",
          hint = "Palaa alkuun",
          action = {
            screen = "home",
          },
        },
      },
    },
  },
}
