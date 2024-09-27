# Use the official Rust base image
FROM rust:latest as builder

# Set the working directory for the Rust app
WORKDIR /app

# Copy the Rust project into the container
COPY . .

# Build the Rust project (this builds the binary named 'rabbit') and capture logs
RUN cargo build --release --verbose || (echo "Cargo build failed" && exit 1)

# Use a lightweight Node.js image for the final stage
FROM node:18

# Set the working directory for Node.js
WORKDIR /app

# Copy the built binary from the Rust builder stage
COPY --from=builder /app/target/release/rabbit /app/rabbit

# Make the binary executable
RUN chmod +x /app/rabbit

COPY src/index.html /app/index.html 

# Create an output directory for generated projects
RUN mkdir /projects

# Expose a port (for Vite development)
EXPOSE 3000

# Define the entrypoint for running your binary 'rabbit'
ENTRYPOINT ["/app/rabbit"]
