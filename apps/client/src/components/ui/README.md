# UI Components

This directory contains reusable UI components built with the PeerUP design system. These components are theme-agnostic and automatically adapt to the active theme.

## Component Categories

### Layout Components
- **Card** - Container with background and border
- **CardHeader** - Header section for cards
- **CardBody** - Content area for cards

### Form Components
- **Input** - Text input with various types
- **Select** - Dropdown select input
- **Textarea** - Multi-line text input
- **Checkbox** - Checkbox input
- **Label** - Form labels with required indicators
- **FormGroup** - Container for form fields with consistent spacing

### Interactive Components
- **Button** - Clickable buttons with variants and sizes
- **Alert** - Notification and message display

### Typography Components
- **Heading** - Semantic headings (h1-h6) with customizable styling
- **Paragraph** - Text content with size and color options

### Data Display Components
- **Badge** - Small status indicators
- **StatusBadge** - Specialized badge for service status
- **Metric** - Display key metrics with labels and trends

## Usage Examples

### Basic Form
```astro
<Card>
  <CardHeader>
    <Heading as="h2" size="xl">User Settings</Heading>
  </CardHeader>
  <CardBody>
    <FormGroup>
      <Label for="username" required>Username</Label>
      <Input id="username" type="text" placeholder="Enter username" />
    </FormGroup>
    
    <FormGroup>
      <Label for="email">Email</Label>
      <Input id="email" type="email" placeholder="Enter email" />
    </FormGroup>
    
    <div class="flex gap-2">
      <Button type="submit" variant="primary">Save</Button>
      <Button type="button" variant="secondary">Cancel</Button>
    </div>
  </CardBody>
</Card>
```

### Status Display
```astro
<Card>
  <CardBody>
    <div class="flex items-center justify-between">
      <div>
        <Heading as="h3" size="lg">API Service</Heading>
        <Paragraph color="secondary">Production endpoint</Paragraph>
      </div>
      <StatusBadge status="online" />
    </div>
    
    <div class="grid grid-cols-3 gap-4 mt-4">
      <Metric label="Uptime" value="99.9" unit="%" color="success" />
      <Metric label="Response Time" value="142" unit="ms" />
      <Metric label="Requests" value="1.2K" trend="up" />
    </div>
  </CardBody>
</Card>
```

### Alerts and Messages
```astro
<Alert variant="success" title="Success!" dismissible>
  Your settings have been saved successfully.
</Alert>

<Alert variant="warning" title="Warning">
  This action cannot be undone.
</Alert>

<Alert variant="error">
  Failed to connect to the server. Please try again.
</Alert>
```

## Component Props

### Common Props
Most components accept these common props:
- `class` - Additional CSS classes
- `...rest` - Any additional HTML attributes

### Variant Systems
Many components use consistent variant naming:
- **Button/Badge variants**: `primary`, `secondary`, `danger`, `ghost`
- **Color variants**: `primary`, `secondary`, `tertiary`, `inverse`, `success`, `warning`, `error`, `info`
- **Size variants**: `sm`, `md`, `lg` (some components have `xs`, `xl`, etc.)

## Theme Integration

All components automatically use the active theme via CSS custom properties. The design system provides:

- **Colors**: Semantic color tokens that adapt to the theme
- **Typography**: Consistent font sizes and weights
- **Spacing**: Standardized padding and margins
- **Borders**: Consistent border radius and styles

## Best Practices

1. **Use semantic variants** - Choose color variants based on meaning, not appearance
2. **Consistent spacing** - Use the built-in spacing classes from the design system
3. **Accessible forms** - Always associate labels with inputs using the `for` attribute
4. **Responsive design** - Use grid classes and responsive utilities
5. **Component composition** - Combine simple components to build complex interfaces

## Adding New Components

When creating new components:

1. Follow the existing patterns and prop interfaces
2. Use the design system tokens via CSS custom properties
3. Include TypeScript interfaces for props
4. Support the common props (`class`, `...rest`)
5. Add the component to the index.ts file
6. Update this README with usage examples

## File Structure

```
ui/
├── index.ts              # Component exports
├── README.md            # This file
├── Button.astro         # Button component
├── Card.astro           # Card container
├── CardHeader.astro     # Card header
├── CardBody.astro       # Card body
├── Input.astro          # Text input
├── Select.astro         # Select dropdown
├── Textarea.astro       # Textarea input
├── Checkbox.astro       # Checkbox input
├── Label.astro          # Form label
├── FormGroup.astro      # Form field container
├── Heading.astro        # Headings (h1-h6)
├── Paragraph.astro      # Paragraph text
├── Badge.astro          # Status badges
├── StatusBadge.astro    # Service status badges
├── Metric.astro         # Metric display
└── Alert.astro          # Alert messages
```
