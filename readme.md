## Ambilight with WLED on Windows

**[Work in progress]**

[Ambilight "Bias lighting" wiki](https://en.wikipedia.org/wiki/Bias_lighting)

Yet another Ambilight solution. This time for Windows using very fast and performant desktop duplication WinAPI.

## Main Goals

- [x] On **4K 120Hz** display use only **less than 3%** of GPU/CPU
- [x] WLED UDP Realtime procotol
- [x] V-Sync or not-very-precise FPS limitter
- [ ] HDR support
- [x] Use DirectX DXGI desktop duplication
- [x] Use DirectX texture mipmaps for quick calculation of average colors on GPU
- [ ] Execute second resize on GPU 
- [ ] Make it pretty
