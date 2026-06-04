<h1 align="center">WebLayer</h1>

<p align="center"><strong>Control the web you consume.</strong></p>

WebLayer helps you take sovereignty over the web you consume. It sends page content from your browser to a local daemon, where your own rules and AI agents can learn from your feedback, hide what you do not want to see, and keep relevant browsing data under your control.

As you browse, WebLayer saves the posts you encounter locally so your agents can later search, inspect, and reason over what you have seen. The local dashboard shows captured posts, rule activity, feedback, and rule proposals. Your active rule set can be curated manually or automatically, and WebLayer can learn from the posts you dislike so it gets better at filtering the material you do not want in your feed.

<p align="center">
  <img src="assets/architecture.svg" alt="Browser to WebLayer extension to WebLayer daemon, with an AI agent and content store connected to the daemon" width="720">
</p>

## Documentation

Documentation is published at <https://rndhouse.github.io/weblayer/>.

## Supported Sites

### X.com

WebLayer currently supports X.com posts, adding a local dislike control so you can hide posts and teach the daemon what you do not want to see. Hidden posts are replaced with a compact placeholder, so you can still see that WebLayer acted and expand the post when you want to inspect it.

<p align="center">
  <img src="assets/x-dislike-post-visible.jpg" alt="WebLayer dislike control on a visible X.com post" width="520">
</p>

<p align="center">
  <img src="assets/x-hidden-post-example.png" alt="WebLayer hidden post placeholder on X.com" width="520">
</p>
