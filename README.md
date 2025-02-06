# Distributed System for User, Admin, and Game Services

This project implements a distributed system comprising three services: **User Service**, **Admin Service**, and **Game Service**. An **API Gateway** handles request routing between these services. The system incorporates **Redis** for session management, **Kafka** for inter-service communication, **Elasticsearch** for game retrieval, and uses **Mockaroo** to generate dummy data for testing.

## Overview

This system consists of three main services:
- **User Service**: Manages user registration, login, and sessions.
- **Admin Service**: Handles admin authentication, user managementand game management.
- **Game Service**: Manages user interactions with games and retrieves game data.

The system also uses **Redis** for session management, **Kafka** for event-driven communication between services, and **Elasticsearch** for fast game data retrieval. 

## Service Descriptions

### User Service

**API Endpoints**:
- `POST /register`: Register a new user and return JWT token
- `POST /login`: Authenticate user and return JWT token
- `POST /logout`: End user session
- `GET /view_user`: Fetch user profile
- `POST /update`: Update user data
- `GET /verify-email`: User email verification
- `GET /resend-verification`: Re-send verification mail

-----

### Admin Service

**API Endpoints**:
- `POST /admin/register`: Register a new admin
- `POST /admin/login`: Authenticate admin and return JWT token
- `POST /logout`: End admin session
- `POST /game/create`: Create a new game
- `GET /game/{slug}`: Get a game by slug
- `PUT /game/update`: Update game details
- `DELETE /game/delete`: Remove a game
- `GET /users`: Fetch user data
- `GET /users/{id}`: Fetch user by id
- `DELETE /users/{id}`: Delete user by id

-----

### Game Service

**API Endpoints**:
- `GET /games`: Fetch games based on activity
- `POST /games/rate`: Rate a game

-----

## API Gateway

The **API Gateway** is responsible for routing incoming requests to the appropriate service based on the endpoint. It also handles central authentication and validation.

**Example Routing**:
- `/user/*` → User Service
- `/admin/*` → Admin Service
- `/games/*` → Game Service
