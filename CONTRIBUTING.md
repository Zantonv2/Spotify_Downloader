# Contributing to Spotify Downloader

Thank you for your interest in contributing to Spotify Downloader! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites
- Rust (latest stable version)
- Node.js (v16 or higher)
- Git
- FFmpeg (for audio processing)
- Python 3.8+ (for metadata embedding)

### Development Setup

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/ZantonV2/Spotify_Downloader.git
   cd Spotify_Downloader
   ```

2. **Install dependencies**
   ```bash
   # Install Rust dependencies
   cargo build
   
   # Install Node.js dependencies
   npm install
   ```

3. **Run the development server**
   ```bash
   npm run tauri dev
   ```

## 📝 How to Contribute

### Reporting Issues
- Use the GitHub issue tracker
- Provide detailed information about the bug
- Include steps to reproduce the issue
- Attach relevant logs or screenshots

### Suggesting Features
- Open a new issue with the "enhancement" label
- Describe the feature in detail
- Explain why it would be useful
- Consider implementation complexity

### Code Contributions

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation if needed

3. **Test your changes**
   ```bash
   # Run Rust tests
   cargo test
   
   # Run frontend tests
   npm test
   
   # Build the application
   npm run tauri build
   ```

4. **Commit your changes**
   ```bash
   git commit -m "Add: brief description of changes"
   ```

5. **Push and create a Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```

## 🎨 Code Style Guidelines

### Rust Code
- Follow Rust naming conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` for linting
- Document public functions with `///`
- Handle errors gracefully with `Result<T, E>`

### TypeScript/React Code
- Use TypeScript for type safety
- Follow React best practices
- Use functional components with hooks
- Use Tailwind CSS for styling
- Keep components small and focused

### General Guidelines
- Write clear, self-documenting code
- Add comments for complex logic
- Use meaningful variable and function names
- Keep functions small and focused
- Follow the existing project structure

## 🧪 Testing

### Running Tests
```bash
# Run all tests
npm run test

# Run specific test suites
cargo test --test test_name
npm test -- --testNamePattern="test_name"
```

### Test Coverage
- Aim for high test coverage
- Test both success and error cases
- Include integration tests for critical paths
- Test UI components with user interactions

## 📚 Documentation

### Code Documentation
- Document all public APIs
- Include usage examples
- Update README.md for new features
- Add inline comments for complex logic

### User Documentation
- Update README.md for new features
- Add screenshots for UI changes
- Document configuration options
- Provide troubleshooting guides

## 🐛 Bug Reports

When reporting bugs, please include:

1. **Environment Information**
   - Operating System
   - Rust version
   - Node.js version
   - FFmpeg version

2. **Steps to Reproduce**
   - Clear, numbered steps
   - Expected behavior
   - Actual behavior

3. **Additional Information**
   - Error messages
   - Log files
   - Screenshots
   - Sample files (if applicable)

## 🎯 Feature Requests

When suggesting features:

1. **Describe the Feature**
   - What should it do?
   - Why is it needed?
   - How should it work?

2. **Consider Implementation**
   - Is it technically feasible?
   - Does it fit the project's scope?
   - Are there any dependencies?

3. **Provide Context**
   - Use cases
   - User stories
   - Mockups or examples

## 🔄 Pull Request Process

1. **Before Submitting**
   - Ensure all tests pass
   - Update documentation
   - Follow code style guidelines
   - Rebase on latest main branch

2. **Pull Request Template**
   - Clear title and description
   - Link to related issues
   - List changes made
   - Include screenshots for UI changes

3. **Review Process**
   - Address review comments
   - Make requested changes
   - Respond to feedback
   - Keep PRs focused and small

## 📋 Project Structure

```
src-tauri/
├── src/
│   ├── commands.rs          # Tauri commands
│   ├── downloader/          # Download strategies
│   ├── metadata/            # Metadata providers
│   ├── search/              # Search functionality
│   └── utils.rs             # Utility functions
src/
├── components/              # React components
├── hooks/                   # Custom React hooks
├── types/                   # TypeScript definitions
└── App.tsx                  # Main application
```

## 🤝 Community Guidelines

- Be respectful and inclusive
- Help others learn and grow
- Provide constructive feedback
- Follow the code of conduct
- Celebrate contributions

## 📞 Getting Help

- Check existing issues and discussions
- Join our community discussions
- Ask questions in issues
- Reach out to maintainers

Thank you for contributing to Spotify Downloader! 🎵
