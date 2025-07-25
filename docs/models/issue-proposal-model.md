# Issue Proposal Model

## Overview

This document defines the standard model for proposing issues in the PowerTo platform. Following this model ensures that all necessary information is provided for community members to understand, evaluate, and vote on proposed issues.

## Issue Proposal Structure

### Basic Information

| Field | Description | Required | Type |
|-------|-------------|----------|------|
| Title | A concise, descriptive title for the issue | Yes | String (max 100 chars) |
| Category | The category that best describes the issue | Yes | Enum (Infrastructure, Public Safety, Environment, Education, Healthcare, Transportation, Other) |
| Location | The specific location affected by the issue | Yes | String |
| Summary | A brief summary of the issue | Yes | String (max 250 chars) |

### Detailed Description

| Field | Description | Required | Type |
|-------|-------------|----------|------|
| Problem Statement | Detailed description of the problem | Yes | Text |
| Affected Community | Description of who is affected by this issue | Yes | Text |
| Current Situation | Description of the current state | Yes | Text |
| Desired Outcome | Description of the desired outcome if the issue is resolved | Yes | Text |
| Proposed Solution | Optional suggestion for how to address the issue | No | Text |
| Timeline Considerations | Any time constraints or considerations | No | Text |

### Supporting Information

| Field | Description | Required | Type |
|-------|-------------|----------|------|
| Images | Photos or images that illustrate the issue | No | File Upload (max 5) |
| Documents | Supporting documents or references | No | File Upload (max 3) |
| External Links | Links to relevant external resources | No | URLs (max 5) |
| Estimated Impact | Estimated number of people affected | Yes | Number |
| Previous Attempts | Description of any previous attempts to address this issue | No | Text |

### Contact Information

| Field | Description | Required | Type |
|-------|-------------|----------|------|
| Proposer Name | Name of the person proposing the issue | Yes | String |
| Contact Email | Email for follow-up questions | Yes | Email |
| Organization | Organization the proposer represents (if any) | No | String |
| Public Contact | Whether contact information can be made public | Yes | Boolean |

## Example Issue Proposal

### Basic Information
- **Title**: Dangerous Intersection at Main St. and Oak Ave.
- **Category**: Transportation
- **Location**: Intersection of Main Street and Oak Avenue
- **Summary**: The intersection lacks proper traffic signals and has poor visibility, resulting in frequent near-misses and three accidents in the past year.

### Detailed Description
- **Problem Statement**: The intersection of Main Street and Oak Avenue has become increasingly dangerous due to increased traffic volume, poor visibility due to overgrown vegetation, and lack of proper traffic signals. Currently, there are only stop signs on Oak Avenue, while Main Street traffic does not stop.
- **Affected Community**: All residents who use this intersection, particularly the 500+ families in the Oak Hill neighborhood who must use this route daily, as well as students walking to Lincoln Elementary School.
- **Current Situation**: The intersection has stop signs only on Oak Avenue. Visibility is limited by overgrown vegetation on the northeast corner. There have been three reported accidents in the past year and numerous near-misses.
- **Desired Outcome**: A safe intersection with proper traffic control and good visibility that prevents accidents and allows for safe pedestrian crossing.
- **Proposed Solution**: Install a four-way stop or traffic light, trim vegetation to improve visibility, and add pedestrian crosswalks with signage.
- **Timeline Considerations**: School year begins in September, so ideally improvements would be completed before then to ensure student safety.

### Supporting Information
- **Images**: [Photos of the intersection from different angles]
- **Documents**: [Traffic incident report from local police department]
- **External Links**: [Link to news article about recent accident]
- **Estimated Impact**: Approximately 2,000 people use this intersection daily
- **Previous Attempts**: A request was submitted to the city transportation department in 2022 but was not prioritized due to budget constraints.

### Contact Information
- **Proposer Name**: Jane Smith
- **Contact Email**: jane.smith@example.com
- **Organization**: Oak Hill Neighborhood Association
- **Public Contact**: Yes

## Submission Process

1. Complete all required fields in the issue proposal form
2. Upload any supporting images or documents
3. Review your submission for completeness and accuracy
4. Submit the proposal for community review
5. Respond to any clarification questions from the community or moderators

## After Submission

Once submitted, your issue proposal will:

1. Be reviewed by moderators for completeness and appropriateness
2. Be published to the community for voting and discussion
3. Receive a unique tracking ID for reference
4. Enter the voting phase where community members can vote and comment
   - Votes from citizens who share the same street, district, or state as the reported issue are given higher priority
   - This ensures that those most directly affected by the issue have the strongest voice in the prioritization process
5. Be prioritized based on community votes, with emphasis on votes from directly affected citizens
6. If highly prioritized, undergo cost analysis and feasibility assessment

## Guidelines for Effective Issue Proposals

1. **Be specific**: Clearly define the problem and affected area
2. **Be objective**: Present facts rather than opinions
3. **Be comprehensive**: Include all relevant information
4. **Be solution-oriented**: Focus on outcomes rather than blame
5. **Be community-minded**: Consider the broader impact on the community
6. **Be realistic**: Consider practical constraints and feasibility
