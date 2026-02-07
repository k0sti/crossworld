# Trellis Integration

Rust client library and Python inference server for [Trellis](https://github.com/microsoft/TRELLIS), Microsoft's text/image-to-3D model. Enables AI-powered voxel generation in Crossworld.

## Overview

Trellis is a unified 3D generative model that supports both text-to-3D and image-to-3D generation. This crate provides:

- **HTTP Client**: Async Rust client for Trellis inference server
- **Server Integration**: Python FastAPI server wrapper for Trellis
- **Format Conversion**: Automatic conversion from Trellis output to Crossworld CSM format
- **CLI Tool**: Command-line interface for generating voxel models

## Prerequisites

### Hardware Requirements

- **GPU**: NVIDIA GPU with CUDA support (RTX 30-series or newer recommended)
- **VRAM**: 8GB minimum (16GB+ recommended)
- **RAM**: 16GB minimum, 32GB+ recommended

### Software Requirements

- **Linux/macOS**: Ubuntu 20.04+, macOS 12+ (Windows via WSL2)
- **CUDA**: 11.8 or newer
- **Python**: 3.10+
- **Rust**: 1.75+

## Server Setup

*Note: Server setup instructions will be added in a future task.*

## Example Usage

```bash
# Check server health
cargo run -p trellis -- health --server http://localhost:3642

# Generate voxel model from text (to be implemented)
# cargo run -p trellis -- generate "a wooden chair" -o chair.csm
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Crossworld Integration                  │
├─────────────────────────────────────────────────────────┤
│  trellis crate (Rust)                                    │
│  ├── TrellisClient: HTTP client for inference server    │
│  ├── types: Request/response types                      │
│  └── convert: Model → CSM/voxel conversion              │
├─────────────────────────────────────────────────────────┤
│  Trellis Server (Python)                                 │
│  ├── FastAPI REST API                                   │
│  ├── Text/image encoding                                │
│  └── Trellis model inference                            │
├─────────────────────────────────────────────────────────┤
│  Trellis (Microsoft)                                     │
│  └── Unified 3D generative model                        │
└─────────────────────────────────────────────────────────┘
```

## Development Status

This crate is currently under development. The following components are planned:

- [ ] Client module (`src/client.rs`)
- [ ] Types module (`src/types.rs`)
- [ ] Conversion module (`src/convert.rs`)
- [ ] Python inference server
- [ ] CLI implementation
- [ ] Documentation and examples

## References

- [Trellis Paper](https://arxiv.org/abs/2412.01506) - Microsoft Research
- [Trellis Repository](https://github.com/microsoft/TRELLIS) - Official code
- [Model Weights](https://huggingface.co/JeffreyXiang/TRELLIS-image-large) - Hugging Face
