export default grammar({
    name: "froggy",

    extras: $ => [
        /\s/,
        $.comment
    ],

    rules: {
        program: $ => repeat($.statement),

        statement: $ => choice(
            $.stack_operation,
            $.control_flow,
            $.stack_manipulation,
            $.arithmetic,
            $.comparison,
            $.label_definition
        ),


        stack_operation: $ => choice(
            $.ribbit,
            $.croak,
            $.plop,
            $.splash,
            $.gulp,
            $.burp
        ),

        ribbit: _ => 'RIBBIT',
        croak: _ => 'CROAK',
        splash: _ => 'SPLASH',
        gulp: _ => 'GULP',
        burp: _ => 'BURP',

        plop: $ => seq(
            'PLOP',
            choice(
                $.number,
                $.string
            )
        ),

        control_flow: $ => choice(
            $.hop,
            $.leap
        ),


        hop: $ => seq(
            'HOP',
            $.identifier
        ),

        leap: $ => seq(
            'LEAP',
            $.identifier
        ),

        label_definition: $ => seq(
            'LILY',
            $.identifier
        ),

        stack_manipulation: $ => choice(
            $.dup,
            $.swap,
            $.over
        ),

        dup: _ => 'DUP',
        swap: _ => 'SWAP',
        over: _ => 'OVER',

        arithmetic: $ => choice(
            $.add,
            $.sub,
            $.mul,
            $.div
        ),

        add: _ => 'ADD',
        sub: _ => 'SUB',
        mul: _ => 'MUL',
        div: _ => 'DIV',

        comparison: $ => choice(
            $.equals,
            $.not_equal,
            $.less_than,
            $.greater_than,
            $.less_eq,
            $.greater_eq
        ),

        equals: _ => 'EQUALS',
        not_equal: _ => 'NOT_EQUAL',
        less_than: _ => 'LESS_THAN',
        greater_than: _ => 'GREATER_THAN',
        less_eq: _ => 'LESS_EQ',
        greater_eq: _ => 'GREATER_EQ',

        // Literals
        number: _ => /[0-9]+(\.[0-9]+)?/,

        string: _ => /"([^"\\]|\\.)*"/,

        identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,

        comment: _ => token(seq('//', /.*/))

    }
});
